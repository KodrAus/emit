/*!
Sampling metrics.

Metrics are an effective approach to monitoring applications at scale. They can be cheap to collect, making them suitable for performance sensitive operations. They can also be compact to report, making them suitable for high-volume scenarios. `emit` doesn't provide any infrastructure for collecting or sampling metrics. What it does provide is a standard way to report metric samples as events.

A standard kind of metric is a monotonic counter, which can be represented as an atomic integer. In this example, our counter is for the number of bytes written to a file, which we'll call `bytes_written`. We can report a sample of this counter as an event by wrapping it in a [`Metric`]:

```
# fn sample_bytes_written() -> usize { 4643 }
use emit::{well_known::METRIC_AGG_COUNT, Clock};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let sample = sample_bytes_written();

rt.emit(
    emit::Metric::new(
        emit::module!(),
        now,
        "bytes_written",
        METRIC_AGG_COUNT,
        sample,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-29T10:08:24.780230000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 4643,
    },
}
```

Metrics may also be emitted manually:

```
# fn sample_bytes_written() -> usize { 4643 }
use emit::well_known::{EVENT_KIND_METRIC, METRIC_AGG_COUNT};

let sample = sample_bytes_written();

emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    event_kind: EVENT_KIND_METRIC,
    metric_agg: METRIC_AGG_COUNT,
    metric_name: "bytes_written",
    metric_value: sample,
);
```

The data model of metrics is an extension of `emit`'s events. Metric events are points or buckets in a time-series. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following well-known properties:

- `event_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: A monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: A potentially non-monotonic sum of defined values.
    - `"min"`: The lowest ordered value.
    - `"max"`: The largest ordered value.
    - `"last"`: The most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.

### Cumulative metrics

Metric events with a point extent are cumulative. Their `metric_value` is the result of applying their `metric_agg` over the entire underlying stream up to that point.

The following metric reports the current number of bytes written as 591:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();

rt.emit(
    emit::Metric::new(
        emit::module!(),
        now,
        "bytes_written",
        METRIC_AGG_COUNT,
        591,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:53:41.069203000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 591,
    },
}
```

### Delta metrics

Metric events with a span extent are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

rt.emit(
    emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        17,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:55:59.839770000Z".."2024-04-30T06:56:29.839770000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 17,
    },
}
```

### Time-series metrics

Metric events with a span extent, where the `metric_value` is an array are a complete time-series. Each element in the array is a bucket in the time-series. The width of each bucket is the length of the extent divided by the number of buckets.

The following metric is for a time-series with 15 buckets, where each bucket covers 1 second:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(15));

rt.emit(
    emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        &[
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T07:03:07.828185000Z".."2024-04-30T07:03:22.828185000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": [
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
    },
}
```
*/

use core::ops::ControlFlow;

use emit_core::{
    and::And,
    emitter::Emitter,
    event::{Event, ToEvent},
    extent::{Extent, ToExtent},
    or::Or,
    path::Path,
    props::{ErasedProps, Props},
    str::{Str, ToStr},
    template::{self, Template},
    value::{ToValue, Value},
    well_known::{KEY_EVENT_KIND, KEY_METRIC_AGG, KEY_METRIC_NAME, KEY_METRIC_VALUE},
};

use crate::kind::Kind;

pub struct Metric<'a, P> {
    module: Path<'a>,
    extent: Option<Extent>,
    name: Str<'a>,
    agg: Str<'a>,
    value: Value<'a>,
    props: P,
}

impl<'a, P> Metric<'a, P> {
    pub fn new(
        module: impl Into<Path<'a>>,
        extent: impl ToExtent,
        name: impl Into<Str<'a>>,
        agg: impl Into<Str<'a>>,
        value: impl Into<Value<'a>>,
        props: P,
    ) -> Self {
        Metric {
            module: module.into(),
            extent: extent.to_extent(),
            name: name.into(),
            agg: agg.into(),
            value: value.into(),
            props,
        }
    }

    pub fn module(&self) -> &Path<'a> {
        &self.module
    }

    pub fn with_module(mut self, module: impl Into<Path<'a>>) -> Self {
        self.module = module.into();
        self
    }

    pub fn name(&self) -> &Str<'a> {
        &self.name
    }

    pub fn with_name(mut self, name: impl Into<Str<'a>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn agg(&self) -> &Str<'a> {
        &self.agg
    }

    pub fn with_agg(mut self, agg: impl Into<Str<'a>>) -> Self {
        self.agg = agg.into();
        self
    }

    pub fn value(&self) -> &Value<'a> {
        &self.value
    }

    pub fn with_value(mut self, value: impl Into<Value<'a>>) -> Self {
        self.value = value.into();
        self
    }

    pub fn with_extent(mut self, extent: impl ToExtent) -> Self {
        self.extent = extent.to_extent();
        self
    }

    pub fn extent(&self) -> &Option<Extent> {
        &self.extent
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn with_props<U>(self, props: U) -> Metric<'a, U> {
        Metric {
            module: self.module,
            extent: self.extent,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }
}

impl<'a, P: Props> ToEvent for Metric<'a, P> {
    type Props<'b> = &'b Self where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<Self::Props<'b>> {
        // "{metric_agg} of {metric_name} is {metric_value}"
        const TEMPLATE: &'static [template::Part<'static>] = &[
            template::Part::hole("metric_agg"),
            template::Part::text(" of "),
            template::Part::hole("metric_name"),
            template::Part::text(" is "),
            template::Part::hole("metric_value"),
        ];

        Event::new(
            self.module.by_ref(),
            self.extent.clone(),
            Template::new(TEMPLATE),
            self,
        )
    }
}

impl<'a, P: Props> Metric<'a, P> {
    pub fn by_ref<'b>(&'b self) -> Metric<'b, &'b P> {
        Metric {
            module: self.module.by_ref(),
            extent: self.extent.clone(),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: &self.props,
        }
    }

    pub fn erase<'b>(&'b self) -> Metric<'b, &'b dyn ErasedProps> {
        Metric {
            module: self.module.by_ref(),
            extent: self.extent.clone(),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: &self.props,
        }
    }
}

impl<'a, P> ToExtent for Metric<'a, P> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent.clone()
    }
}

impl<'a, P: Props> Props for Metric<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_EVENT_KIND.to_str(), Kind::Metric.to_value())?;
        for_each(KEY_METRIC_NAME.to_str(), self.name.to_value())?;
        for_each(KEY_METRIC_AGG.to_str(), self.agg.to_value())?;
        for_each(KEY_METRIC_VALUE.to_str(), self.value.by_ref())?;

        self.props.for_each(for_each)
    }
}

pub trait Source {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S);

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        struct FromEmitter<E>(E);

        impl<E: Emitter> sampler::Sampler for FromEmitter<E> {
            fn metric<P: Props>(&self, metric: &Metric<P>) {
                self.0.emit(metric)
            }
        }

        self.sample_metrics(FromEmitter(emitter))
    }

    fn and_sample<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    #[cfg(feature = "alloc")]
    fn report_to(self, reporter: &mut Reporter) -> Self
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        reporter.source(self.clone());

        self
    }
}

impl<'a, T: Source + ?Sized> Source for &'a T {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        (**self).sample_metrics(sampler)
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        (**self).emit_metrics(emitter)
    }
}

impl<T: Source> Source for Option<T> {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        if let Some(source) = self {
            source.sample_metrics(sampler);
        }
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        if let Some(source) = self {
            source.emit_metrics(emitter);
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Source + ?Sized + 'a> Source for alloc::boxed::Box<T> {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        (**self).sample_metrics(sampler)
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        (**self).emit_metrics(emitter)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Source + ?Sized + 'a> Source for alloc::sync::Arc<T> {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        (**self).sample_metrics(sampler)
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        (**self).emit_metrics(emitter)
    }
}

impl<T: Source, U: Source> Source for And<T, U> {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        self.left().sample_metrics(&sampler);
        self.right().sample_metrics(&sampler);
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        self.left().emit_metrics(&emitter);
        self.right().emit_metrics(&emitter);
    }
}

impl<T: Source, U: Source> Source for Or<T, U> {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        self.left().sample_metrics(&sampler);
        self.right().sample_metrics(&sampler);
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        self.left().emit_metrics(&emitter);
        self.right().emit_metrics(&emitter);
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::{boxed::Box, vec::Vec};

    pub struct Reporter(Vec<Box<dyn ErasedSource + Send + Sync>>);

    impl Reporter {
        pub const fn new() -> Self {
            Reporter(Vec::new())
        }

        pub fn source(&mut self, source: impl Source + Send + Sync + 'static) -> &mut Self {
            self.0.push(Box::new(source));

            self
        }
    }

    impl Source for Reporter {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            for source in &self.0 {
                source.sample_metrics(&sampler);
            }
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            for source in &self.0 {
                source.emit_metrics(&emitter);
            }
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

mod internal {
    use emit_core::emitter;

    use super::*;

    pub trait DispatchSource {
        fn dispatch_sample_metrics(&self, sampler: &dyn sampler::ErasedSampler);

        fn dispatch_emit_metrics(&self, emitter: &dyn emitter::ErasedEmitter);
    }

    pub trait SealedSource {
        fn erase_source(&self) -> crate::internal::Erased<&dyn DispatchSource>;
    }
}

pub trait ErasedSource: internal::SealedSource {}

impl<T: Source> ErasedSource for T {}

impl<T: Source> internal::SealedSource for T {
    fn erase_source(&self) -> crate::internal::Erased<&dyn internal::DispatchSource> {
        crate::internal::Erased(self)
    }
}

impl<T: Source> internal::DispatchSource for T {
    fn dispatch_sample_metrics(&self, sampler: &dyn sampler::ErasedSampler) {
        self.sample_metrics(sampler)
    }

    fn dispatch_emit_metrics(&self, emitter: &dyn emit_core::emitter::ErasedEmitter) {
        self.emit_metrics(emitter)
    }
}

impl<'a> Source for dyn ErasedSource + 'a {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        self.erase_source().0.dispatch_sample_metrics(&sampler)
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        self.erase_source().0.dispatch_emit_metrics(&emitter)
    }
}

impl<'a> Source for dyn ErasedSource + Send + Sync + 'a {
    fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
        (self as &(dyn ErasedSource + 'a)).sample_metrics(sampler)
    }

    fn emit_metrics<E: Emitter>(&self, emitter: E) {
        (self as &(dyn ErasedSource + 'a)).emit_metrics(emitter)
    }
}

pub mod sampler {
    use emit_core::empty::Empty;

    use super::*;

    pub trait Sampler {
        fn metric<P: Props>(&self, metric: &Metric<P>);
    }

    impl<'a, T: Sampler + ?Sized> Sampler for &'a T {
        fn metric<P: Props>(&self, metric: &Metric<P>) {
            (**self).metric(metric)
        }
    }

    impl Sampler for Empty {
        fn metric<P: Props>(&self, _: &Metric<P>) {}
    }

    pub fn from_fn<F: Fn(&Metric<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
        FromFn(f)
    }

    pub struct FromFn<F>(F);

    impl<F: Fn(&Metric<&dyn ErasedProps>)> Sampler for FromFn<F> {
        fn metric<P: Props>(&self, metric: &Metric<P>) {
            (self.0)(&metric.erase())
        }
    }

    mod internal {
        use super::*;

        pub trait DispatchSampler {
            fn dispatch_metric(&self, metric: &Metric<&dyn ErasedProps>);
        }

        pub trait SealedSampler {
            fn erase_sampler(&self) -> crate::internal::Erased<&dyn DispatchSampler>;
        }
    }

    pub trait ErasedSampler: internal::SealedSampler {}

    impl<T: Sampler> ErasedSampler for T {}

    impl<T: Sampler> internal::SealedSampler for T {
        fn erase_sampler(&self) -> crate::internal::Erased<&dyn internal::DispatchSampler> {
            crate::internal::Erased(self)
        }
    }

    impl<T: Sampler> internal::DispatchSampler for T {
        fn dispatch_metric(&self, metric: &Metric<&dyn ErasedProps>) {
            self.metric(metric)
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + 'a {
        fn metric<P: Props>(&self, metric: &Metric<P>) {
            self.erase_sampler().0.dispatch_metric(&metric.erase())
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + Send + Sync + 'a {
        fn metric<P: Props>(&self, metric: &Metric<P>) {
            (self as &(dyn ErasedSampler + 'a)).metric(metric)
        }
    }
}
