mod export_metrics_service;
mod metric;

use std::ops::ControlFlow;

pub use self::{export_metrics_service::*, metric::*};

use emit::{
    well_known::{
        METRIC_AGG_COUNT, METRIC_AGG_KEY, METRIC_AGG_SUM, METRIC_NAME_KEY, METRIC_UNIT_KEY,
        METRIC_VALUE_KEY,
    },
    Props,
};
use emit_batcher::BatchError;
use sval::Value;
use sval_protobuf::buf::ProtoBuf;

use super::{MessageFormatter, MessageRenderer, PreEncoded};

pub(crate) struct EventEncoder {
    pub name: Box<MessageFormatter>,
}

impl Default for EventEncoder {
    fn default() -> Self {
        Self {
            name: default_name_formatter(),
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
                        },
                    )
                })
                .unwrap_or((0, 0, AggregationTemporality::Unspecified));

            let metric_name = MessageRenderer {
                fmt: &self.name,
                evt,
            };

            let metric_unit = evt.props().get(METRIC_UNIT_KEY);

            let protobuf = match metric_agg.and_then(|kind| kind.to_cow_str()).as_deref() {
                Some(METRIC_AGG_SUM) => sval_protobuf::stream_to_protobuf(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Sum::<_>(Sum::<_> {
                        aggregation_temporality,
                        is_monotonic: false,
                        data_points: &SumPoints::new().points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
                    }),
                }),
                Some(METRIC_AGG_COUNT) => sval_protobuf::stream_to_protobuf(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Sum::<_>(Sum::<_> {
                        aggregation_temporality,
                        is_monotonic: true,
                        data_points: &SumPoints::new().points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
                    }),
                }),
                _ => sval_protobuf::stream_to_protobuf(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Gauge(Gauge::<_> {
                        data_points: &RawPointSet::new().points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
                    }),
                }),
            };

            return Some(PreEncoded::Proto(protobuf));
        }

        None
    }
}

trait DataPointBuilder {
    type Points;

    fn points_from_value(
        self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
        value: emit::Value<'_>,
    ) -> Option<Self::Points>
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

    fn into_points(self, start_time_unix_nano: u64, time_unix_nano: u64) -> Option<Self::Points>;
}

struct SumPoints(NumberDataPoint<'static>);

impl SumPoints {
    fn new() -> Self {
        SumPoints(NumberDataPoint {
            attributes: &[],
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsInt(AsInt(0)),
        })
    }
}

impl DataPointBuilder for SumPoints {
    type Points = [NumberDataPoint<'static>; 1];

    fn push_point_i64(&mut self, value: i64) {
        self.0.value = match self.0.value {
            NumberDataPointValue::AsInt(AsInt(current)) => current
                .checked_add(value)
                .map(|value| NumberDataPointValue::AsInt(AsInt(value)))
                .unwrap_or(NumberDataPointValue::AsDouble(AsDouble(f64::INFINITY))),
            NumberDataPointValue::AsDouble(AsDouble(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(current + value as f64))
            }
        };
    }

    fn push_point_f64(&mut self, value: f64) {
        self.0.value = match self.0.value {
            NumberDataPointValue::AsInt(AsInt(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(value + current as f64))
            }
            NumberDataPointValue::AsDouble(AsDouble(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(current + value))
            }
        };
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Self::Points> {
        self.0.start_time_unix_nano = start_time_unix_nano;
        self.0.time_unix_nano = time_unix_nano;

        Some([self.0])
    }
}

struct RawPointSet(Vec<NumberDataPoint<'static>>);

impl RawPointSet {
    fn new() -> Self {
        RawPointSet(Vec::new())
    }
}

impl DataPointBuilder for RawPointSet {
    type Points = Vec<NumberDataPoint<'static>>;

    fn push_point_i64(&mut self, value: i64) {
        self.0.push(NumberDataPoint {
            attributes: &[],
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsInt(AsInt(value)),
        });
    }

    fn push_point_f64(&mut self, value: f64) {
        self.0.push(NumberDataPoint {
            attributes: &[],
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsDouble(AsDouble(value)),
        });
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Self::Points> {
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

pub(crate) fn encode_request(
    resource: Option<&PreEncoded>,
    scope: Option<&PreEncoded>,
    metrics: &[PreEncoded],
) -> Result<PreEncoded, BatchError<Vec<PreEncoded>>> {
    Ok(PreEncoded::Proto(sval_protobuf::stream_to_protobuf(
        ExportMetricsServiceRequest {
            resource_metrics: &[ResourceMetrics {
                resource: &resource,
                scope_metrics: &[ScopeMetrics {
                    scope: &scope,
                    metrics,
                    schema_url: "",
                }],
                schema_url: "",
            }],
        },
    )))
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
