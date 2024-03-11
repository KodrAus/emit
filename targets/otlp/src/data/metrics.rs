use std::ops::ControlFlow;

use emit::{
    well_known::{
        METRIC_AGG_COUNT, METRIC_AGG_KEY, METRIC_AGG_SUM, METRIC_NAME_KEY, METRIC_VALUE_KEY,
    },
    Props,
};
use emit_batcher::BatchError;
use sval::Value;
use sval_protobuf::buf::ProtoBuf;

use crate::data::generated::{common::v1::*, metrics::v1::*};

use super::{MessageFormatter, MessageRenderer, PreEncoded};

pub(crate) struct EventEncoder {
    pub name: Box<MessageFormatter>,
    pub guage: GuageEncoder,
    pub sum: SumEncoder,
    pub count: SumEncoder,
}

pub(crate) struct SumEncoder {
    pub single_point_per_sample: bool,
    // TODO: delta to cumulative
}

impl SumEncoder {
    fn encode_data_points(
        &self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
        metric_value: emit::Value,
    ) -> Option<Vec<NumberDataPoint>> {
        if self.single_point_per_sample {
            SumPoints::new().points_from_value(start_time_unix_nano, time_unix_nano, metric_value)
        } else {
            RawPointSet::new().points_from_value(start_time_unix_nano, time_unix_nano, metric_value)
        }
    }
}

pub(crate) struct GuageEncoder {}

impl GuageEncoder {
    fn encode_data_points(
        &self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
        metric_value: emit::Value,
    ) -> Option<Vec<NumberDataPoint>> {
        RawPointSet::new().points_from_value(start_time_unix_nano, time_unix_nano, metric_value)
    }
}

trait DataPointBuilder {
    fn points_from_value(
        self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
        value: emit::Value,
    ) -> Option<Vec<NumberDataPoint>>
    where
        Self: Sized,
    {
        struct Extract<A> {
            in_seq: bool,
            aggregator: A,
        }

        impl<'sval, A: DataPointBuilder> sval::Stream<'sval> for Extract<A> {
            fn null(&mut self) -> sval::Result {
                sval::error()
            }

            fn bool(&mut self, _: bool) -> sval::Result {
                sval::error()
            }

            fn text_begin(&mut self, _: Option<usize>) -> sval::Result {
                sval::error()
            }

            fn text_fragment_computed(&mut self, _: &str) -> sval::Result {
                sval::error()
            }

            fn text_end(&mut self) -> sval::Result {
                sval::error()
            }

            fn i64(&mut self, value: i64) -> sval::Result {
                self.aggregator.push_point_i64(value);

                Ok(())
            }

            fn f64(&mut self, value: f64) -> sval::Result {
                self.aggregator.push_point_f64(value);

                Ok(())
            }

            fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
                if self.in_seq {
                    return sval::error();
                }

                self.in_seq = true;

                Ok(())
            }

            fn seq_value_begin(&mut self) -> sval::Result {
                Ok(())
            }

            fn seq_value_end(&mut self) -> sval::Result {
                Ok(())
            }

            fn seq_end(&mut self) -> sval::Result {
                self.in_seq = false;

                Ok(())
            }
        }

        let mut extract = Extract {
            in_seq: false,
            aggregator: self,
        };
        value.stream(&mut extract).ok()?;

        extract
            .aggregator
            .into_points(start_time_unix_nano, time_unix_nano)
    }

    fn push_point_i64(&mut self, value: i64);
    fn push_point_f64(&mut self, value: f64);

    fn into_points(
        self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Vec<NumberDataPoint>>;
}

struct SumPoints(NumberDataPoint);

impl SumPoints {
    fn new() -> Self {
        SumPoints(NumberDataPoint {
            attributes: Vec::new(),
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            exemplars: Vec::new(),
            flags: Default::default(),
            value: Some(number_data_point::Value::AsInt(0)),
        })
    }
}

impl DataPointBuilder for SumPoints {
    fn push_point_i64(&mut self, value: i64) {
        self.0.value = match self.0.value {
            Some(number_data_point::Value::AsInt(current)) => current
                .checked_add(value)
                .map(number_data_point::Value::AsInt),
            Some(number_data_point::Value::AsDouble(current)) => {
                Some(number_data_point::Value::AsDouble(current + value as f64))
            }
            None => None,
        };
    }

    fn push_point_f64(&mut self, value: f64) {
        self.0.value = match self.0.value {
            Some(number_data_point::Value::AsInt(current)) => {
                Some(number_data_point::Value::AsDouble(value + current as f64))
            }
            Some(number_data_point::Value::AsDouble(current)) => {
                Some(number_data_point::Value::AsDouble(current + value))
            }
            None => None,
        };
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Vec<NumberDataPoint>> {
        self.0.start_time_unix_nano = start_time_unix_nano;
        self.0.time_unix_nano = time_unix_nano;

        Some(vec![self.0])
    }
}

struct RawPointSet(Vec<NumberDataPoint>);

impl RawPointSet {
    fn new() -> Self {
        RawPointSet(Vec::new())
    }
}

impl DataPointBuilder for RawPointSet {
    fn push_point_i64(&mut self, value: i64) {
        self.0.push(NumberDataPoint {
            attributes: Vec::new(),
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            exemplars: Vec::new(),
            flags: Default::default(),
            value: Some(number_data_point::Value::AsInt(value)),
        });
    }

    fn push_point_f64(&mut self, value: f64) {
        self.0.push(NumberDataPoint {
            attributes: Vec::new(),
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            exemplars: Vec::new(),
            flags: Default::default(),
            value: Some(number_data_point::Value::AsDouble(value)),
        });
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Vec<NumberDataPoint>> {
        match self.0.len() as u64 {
            0 => None,
            1 => {
                self.0[0].start_time_unix_nano = start_time_unix_nano;
                self.0[0].time_unix_nano = time_unix_nano;

                Some(self.0)
            }
            points => {
                let point_time_range = time_unix_nano.saturating_sub(start_time_unix_nano);
                let step = point_time_range / points;

                let mut point_time = start_time_unix_nano;
                for point in &mut self.0 {
                    point.start_time_unix_nano = point_time;
                    point_time += step;
                    point.time_unix_nano = point_time;
                }

                Some(self.0)
            }
        }
    }
}

impl Default for EventEncoder {
    fn default() -> Self {
        Self {
            name: default_name_formatter(),
            guage: GuageEncoder {},
            sum: SumEncoder {
                single_point_per_sample: false,
            },
            count: SumEncoder {
                single_point_per_sample: false,
            },
        }
    }
}

fn default_name_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| {
        if let Some(name) = evt.props().get(METRIC_NAME_KEY) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", evt.msg())
        }
    })
}

impl EventEncoder {
    pub(crate) fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<PreEncoded> {
        use prost::Message;

        if let (Some(metric_value), metric_agg) = (
            evt.props().get(METRIC_VALUE_KEY),
            evt.props().get(METRIC_AGG_KEY),
        ) {
            let (start_time_unix_nano, time_unix_nano, aggregation_temporality) = evt
                .extent()
                .map(|extent| {
                    let range = extent.as_range();

                    (
                        range.start.to_unix_time().as_nanos() as u64,
                        range.end.to_unix_time().as_nanos() as u64,
                        if extent.is_span() {
                            AggregationTemporality::Delta
                        } else {
                            AggregationTemporality::Cumulative
                        } as i32,
                    )
                })
                .unwrap_or_default();

            let metric_name = MessageRenderer {
                fmt: &self.name,
                evt,
            };

            let mut attributes = Vec::new();

            evt.props()
                .filter(|k, _| k != METRIC_NAME_KEY && k != METRIC_VALUE_KEY)
                .for_each(|k, v| {
                    let key = k.to_cow().into_owned();
                    let value = crate::data::generated::any_value::to_value(v);

                    attributes.push(KeyValue { key, value });

                    ControlFlow::Continue(())
                });

            let data = match metric_agg.and_then(|kind| kind.to_cow_str()).as_deref() {
                Some(METRIC_AGG_SUM) => Some(metric::Data::Sum(Sum {
                    aggregation_temporality,
                    is_monotonic: false,
                    data_points: self.sum.encode_data_points(
                        start_time_unix_nano,
                        time_unix_nano,
                        metric_value,
                    )?,
                })),
                Some(METRIC_AGG_COUNT) => Some(metric::Data::Sum(Sum {
                    aggregation_temporality,
                    is_monotonic: true,
                    data_points: self.count.encode_data_points(
                        start_time_unix_nano,
                        time_unix_nano,
                        metric_value,
                    )?,
                })),
                _ => Some(metric::Data::Gauge(Gauge {
                    data_points: self.guage.encode_data_points(
                        start_time_unix_nano,
                        time_unix_nano,
                        metric_value,
                    )?,
                })),
            };

            let msg = Metric {
                name: metric_name.to_string(),
                description: String::new(),
                unit: String::new(),
                data,
            };

            let mut buf = Vec::new();
            msg.encode(&mut buf).unwrap();

            return Some(PreEncoded::Proto(ProtoBuf::pre_encoded(buf)));
        }

        None
    }
}

pub(crate) fn encode_request(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    metrics: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    use prost::Message;

    use crate::data::generated::{
        collector::metrics::v1::*, common::v1::*, metrics::v1::*, resource::v1::*,
    };

    let resource = if let Some(resource) = resource {
        Some(Resource::decode(&*resource.to_vec()).unwrap())
    } else {
        None
    };

    let scope = if let Some(scope) = scope {
        Some(InstrumentationScope::decode(&*scope.to_vec()).unwrap())
    } else {
        None
    };

    let metrics = metrics
        .iter()
        .map(|metric| Metric::decode(&*metric.to_vec()).unwrap())
        .collect();

    let msg = ExportMetricsServiceRequest {
        resource_metrics: vec![ResourceMetrics {
            resource,
            scope_metrics: vec![ScopeMetrics {
                scope,
                metrics,
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    };

    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();

    Ok(PreEncoded::Proto(ProtoBuf::pre_encoded(buf)))
}

#[cfg(feature = "decode_responses")]
pub(crate) fn decode_response(body: Result<&[u8], &[u8]>) {
    match body {
        Ok(body) => {
            emit::warn!(
                rt: emit::runtime::internal(),
                "received metrics {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::collector::metrics::v1::ExportMetricsServiceResponse>(body),
            );
        }
        Err(body) => {
            emit::warn!(
                rt: emit::runtime::internal(),
                "received metrics {response}",
                #[emit::as_debug]
                response: crate::data::generated::response::decode::<crate::data::generated::google::rpc::Status>(body),
            );
        }
    }
}
