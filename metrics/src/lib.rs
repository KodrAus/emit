#![feature(proc_macro_hygiene, stmt_expr_attributes)]

use std::{
    borrow::Cow,
    cmp,
    collections::HashMap,
    mem,
    ops::{ControlFlow, Range},
    sync::Mutex,
    time::Duration,
};

use emit::{
    props::{FromProps, Props},
    str::{Str, ToStr},
    value::{FromValue, ToValue, Value},
    well_known::{
        METRIC_KIND_KEY, METRIC_KIND_MAX, METRIC_KIND_MIN, METRIC_KIND_SUM, METRIC_NAME_KEY,
        METRIC_VALUE_KEY,
    },
    Event, Timestamp,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metric<'m, T> {
    name: Str<'m>,
    kind: Str<'m>,
    value: T,
}

impl<'m, V: FromValue<'m>> FromProps<'m> for Metric<'m, V> {
    fn from_props<P: Props + ?Sized>(props: &'m P) -> Option<Self> {
        Some(Metric::new(
            props.get(METRIC_KIND_KEY)?.pull()?,
            props.get(METRIC_NAME_KEY)?.pull()?,
            props.get(METRIC_VALUE_KEY)?.pull()?,
        ))
    }
}

impl<'m, T> Metric<'m, T> {
    pub const fn new(kind: Str<'m>, name: Str<'m>, value: T) -> Self {
        Metric { name, kind, value }
    }

    pub const fn sum(name: Str<'m>, value: T) -> Self {
        Metric::new(Str::new(METRIC_KIND_SUM), name, value)
    }

    pub fn is_sum(&self) -> bool {
        self.kind() == METRIC_KIND_SUM
    }

    pub const fn min(name: Str<'m>, value: T) -> Self {
        Metric::new(Str::new(METRIC_KIND_MIN), name, value)
    }

    pub fn is_min(&self) -> bool {
        self.kind() == METRIC_KIND_MIN
    }

    pub const fn max(name: Str<'m>, value: T) -> Self {
        Metric::new(Str::new(METRIC_KIND_MAX), name, value)
    }

    pub fn is_max(&self) -> bool {
        self.kind() == METRIC_KIND_MAX
    }

    pub const fn name(&self) -> &Str<'m> {
        &self.name
    }

    pub const fn kind(&self) -> &Str<'m> {
        &self.kind
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'m, V: ToValue> Props for Metric<'m, V> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(METRIC_NAME_KEY.to_str(), self.name.to_value())?;
        for_each(METRIC_VALUE_KEY.to_str(), self.value.to_value())?;
        for_each(METRIC_KIND_KEY.to_str(), self.kind.to_value())?;

        ControlFlow::Continue(())
    }
}

pub struct MetricsEmitter<E> {
    collector: Mutex<MetricsCollector>,
    inner: E,
}

impl<E> MetricsEmitter<E> {
    pub fn new(collector: MetricsCollector, emitter: E) -> Self {
        MetricsEmitter {
            collector: Mutex::new(collector),
            inner: emitter,
        }
    }
}

pub fn plot_metrics_by_count<E>(count: usize, emitter: E) -> MetricsEmitter<E> {
    MetricsEmitter::new(MetricsCollector::new(Bucketing::ByCount(count)), emitter)
}

pub fn plot_metrics_by_time<E>(bucket_size: Duration, emitter: E) -> MetricsEmitter<E> {
    MetricsEmitter::new(
        MetricsCollector::new(Bucketing::ByTime(bucket_size)),
        emitter,
    )
}

impl<E: emit::Emitter> emit::Emitter for MetricsEmitter<E> {
    fn emit<P: Props>(&self, evt: &Event<P>) {
        if self.collector.lock().unwrap().record_metric(evt) {
            return;
        }

        self.inner.emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) {
        for (metric_name, histogram) in self.collector.lock().unwrap().take_sums() {
            if histogram.is_empty() {
                continue;
            }

            let metric_kind = METRIC_KIND_SUM;

            let histogram = histogram.compute();
            let x = histogram.timestamp_range();
            let y = histogram.value_range();

            let metric_value = histogram.buckets();
            let metric_value = &metric_value;

            self.inner.emit(&emit::Event::new(
                x,
                emit::tpl!("{metric_kind} of {metric_name} is in the range {#[emit::fmt(\".3\")] min}..={#[emit::fmt(\".3\")] max}"),
                emit::props! {
                    metric_kind,
                    metric_name,
                    #[emit::as_sval]
                    metric_value,
                    min: y.start,
                    max: y.end,
                },
            ))
        }

        self.inner.blocking_flush(timeout)
    }
}

pub struct MetricsCollector {
    bucketing: Bucketing,
    sums: HashMap<Cow<'static, str>, SumHistogram>,
}

#[derive(Debug, Clone, Copy)]
pub enum Bucketing {
    ByTime(Duration),
    ByCount(usize),
}

pub struct Histogram {
    value_range: Range<f64>,
    timestamp_range: Range<Timestamp>,
    bucket_size: Duration,
    buckets: Vec<f64>,
}

impl Histogram {
    pub fn value_range(&self) -> Range<f64> {
        self.value_range.clone()
    }

    pub fn timestamp_range(&self) -> Range<Timestamp> {
        self.timestamp_range.clone()
    }

    pub fn bucket_size(&self) -> Duration {
        self.bucket_size
    }

    pub fn buckets(&self) -> &[f64] {
        &self.buckets
    }
}

#[derive(Debug, Clone)]
pub struct SumHistogram {
    deltas: Vec<SumHistogramDelta>,
    bucketing: Bucketing,
    start: Option<Timestamp>,
    cumulative: f64,
    omitted: usize,
}

#[derive(Debug, Clone)]
struct SumHistogramDelta {
    range: Range<Timestamp>,
    value: f64,
}

impl MetricsCollector {
    pub fn new(bucketing: Bucketing) -> Self {
        MetricsCollector {
            bucketing,
            sums: HashMap::new(),
        }
    }

    pub fn record_metric(&mut self, evt: &Event<impl Props>) -> bool {
        if let (Some(extent), Some(metric)) = (
            evt.extent().and_then(|extent| extent.as_point()),
            evt.props().pull::<Metric<Value>>(),
        ) {
            if metric.is_sum() {
                return self.record_sum_point(
                    metric.name().to_cow(),
                    *extent,
                    metric.value().as_f64(),
                );
            }
        }

        false
    }

    pub fn record_sum_point(
        &mut self,
        metric: impl Into<Cow<'static, str>>,
        timestamp: Timestamp,
        cumulative: f64,
    ) -> bool {
        let entry = self
            .sums
            .entry(metric.into())
            .or_insert_with(|| SumHistogram {
                deltas: Vec::new(),
                bucketing: self.bucketing,
                omitted: 0,
                start: None,
                cumulative: 0.0,
            });

        let from = if let Some(from) = entry.deltas.last().map(|bucket| bucket.range.end) {
            if from >= timestamp {
                entry.omitted += 1;
                return false;
            }

            from
        } else if let Some(start) = entry.start {
            start
        } else {
            entry.start = Some(timestamp);
            return false;
        };

        let value = cumulative - entry.cumulative;

        entry.cumulative = cumulative;
        entry.deltas.push(SumHistogramDelta {
            range: from..timestamp,
            value,
        });

        true
    }

    pub fn record_sum_span(
        &mut self,
        metric: impl Into<Cow<'static, str>>,
        range: Range<Timestamp>,
        value: f64,
    ) -> bool {
        let entry = self
            .sums
            .entry(metric.into())
            .or_insert_with(|| SumHistogram {
                deltas: Vec::new(),
                bucketing: self.bucketing,
                omitted: 0,
                start: None,
                cumulative: 0.0,
            });

        if let Some(from) = entry.deltas.last().map(|bucket| bucket.range.end) {
            if from >= range.start {
                entry.omitted += 1;
                return false;
            }
        }

        entry.cumulative += value;
        entry.deltas.push(SumHistogramDelta { range, value });

        true
    }

    pub fn iter_sums<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a SumHistogram)> + 'a {
        self.sums.iter().map(|(k, v)| (&**k, v))
    }

    pub fn take_sums(&mut self) -> impl Iterator<Item = (Cow<'static, str>, SumHistogram)> {
        mem::take(&mut self.sums).into_iter()
    }
}

impl SumHistogram {
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    pub fn compute(&self) -> Histogram {
        let mut buckets = Vec::new();

        let extent =
            self.deltas.first().unwrap().range.start..self.deltas.last().unwrap().range.end;

        let bucket_size = match self.bucketing {
            Bucketing::ByTime(size) => size.as_nanos(),
            Bucketing::ByCount(nbuckets) => cmp::max(
                1,
                (extent.end.to_unix_time().as_nanos() - extent.start.to_unix_time().as_nanos())
                    / (nbuckets as u128),
            ),
        };

        let extent_start = extent.start.to_unix_time().as_nanos();

        let bucket_start = {
            let diff = extent_start % bucket_size;

            if diff == 0 {
                extent_start
            } else {
                extent_start - diff
            }
        };

        let mut current_bucket_start = bucket_start;
        let mut current_bucket_value = 0.0;

        let mut bucket_min = f64::NAN;
        let mut bucket_max = -f64::NAN;

        let mut push_bucket = |value: f64| {
            buckets.push(value);
            bucket_min = cmp::min_by(value, bucket_min, f64::total_cmp);
            bucket_max = cmp::max_by(value, bucket_max, f64::total_cmp);
        };

        let mut i = 0;
        while i < self.deltas.len() {
            let delta = &self.deltas[i];

            let current_delta_start = delta.range.start.to_unix_time().as_nanos();
            let current_delta_end = delta.range.end.to_unix_time().as_nanos();

            let current_bucket_end = current_bucket_start + bucket_size;

            // Advance buckets to the start of the delta
            if current_delta_start >= current_bucket_end {
                push_bucket(current_bucket_value);

                current_bucket_value = 0.0;
                current_bucket_start = current_bucket_end;
                continue;
            }

            let intersection = (cmp::min(current_bucket_end, current_delta_end) as f64
                - cmp::max(current_bucket_start, current_delta_start) as f64)
                / (current_delta_end as f64 - current_delta_start as f64);

            current_bucket_value += delta.value * intersection;

            // Advance buckets through the delta
            if current_delta_end > current_bucket_end {
                push_bucket(current_bucket_value);

                current_bucket_value = 0.0;
                current_bucket_start = current_bucket_end;
                continue;
            }

            // Advance deltas through the bucket
            i += 1;
        }

        if current_bucket_value != 0.0 {
            push_bucket(current_bucket_value);
        }

        let histogram_size = duration_from_nanos(bucket_size * buckets.len() as u128);
        let bucket_size = duration_from_nanos(bucket_size);

        Histogram {
            value_range: bucket_min..bucket_max,
            timestamp_range: {
                let start = Timestamp::new(duration_from_nanos(bucket_start)).unwrap();

                start..start + histogram_size
            },
            bucket_size,
            buckets,
        }
    }
}

fn duration_from_nanos(nanos: u128) -> Duration {
    let secs = (nanos / 1_000_000_000) as u64;
    let subsecs = (nanos % 1_000_000_000) as u64;

    Duration::from_secs(secs) + Duration::from_nanos(subsecs)
}
