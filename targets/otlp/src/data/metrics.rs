use std::{borrow::Cow, ops::ControlFlow};

use emit::{
    value::FromValue,
    well_known::{METRIC_KIND_KEY, METRIC_KIND_SUM, METRIC_NAME_KEY, METRIC_VALUE_KEY},
    Props,
};
use emit_batcher::BatchError;
use sval_protobuf::buf::ProtoBuf;

use super::{MessageFormatter, PreEncoded};

pub(crate) struct EventEncoder {
    pub name: Box<MessageFormatter>,
}

impl EventEncoder {
    pub(crate) fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<PreEncoded> {
        use prost::Message;

        if let (Some(metric_name), Some(metric_value), metric_kind) = (
            evt.props().get(METRIC_NAME_KEY),
            evt.props().get(METRIC_VALUE_KEY),
            evt.props().get(METRIC_KIND_KEY),
        ) {
            use crate::data::generated::{common::v1::*, metrics::v1::*};

            let metric_value = metric_value.pull::<MetricValue>()?;
            let metric_name = metric_name
                .to_cow_str()
                .unwrap_or_else(|| Cow::Owned(metric_name.to_string()));

            let mut attributes = Vec::new();

            evt.props()
                .filter(|k, _| k != METRIC_NAME_KEY && k != METRIC_VALUE_KEY)
                .for_each(|k, v| {
                    let key = k.to_cow().into_owned();
                    let value = crate::data::generated::any_value::to_value(v);

                    attributes.push(KeyValue { key, value });

                    ControlFlow::Continue(())
                });

            let (time_unix_nano, start_time_unix_nano, aggregation_temporality) = evt
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

            let data_point = NumberDataPoint {
                attributes,
                start_time_unix_nano,
                time_unix_nano,
                exemplars: Vec::new(),
                flags: 0,
                value: Some(match metric_value {
                    MetricValue::F64(value) => number_data_point::Value::AsDouble(value),
                    MetricValue::I64(value) => number_data_point::Value::AsInt(value),
                }),
            };

            let data = match metric_kind.and_then(|kind| kind.to_cow_str()).as_deref() {
                Some(METRIC_KIND_SUM) => Some(metric::Data::Sum(Sum {
                    aggregation_temporality,
                    is_monotonic: false,
                    data_points: vec![data_point],
                })),
                _ => Some(metric::Data::Gauge(Gauge {
                    data_points: vec![data_point],
                })),
            };

            let msg = Metric {
                name: metric_name.into_owned(),
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

enum MetricValue {
    F64(f64),
    I64(i64),
}

impl<'v> FromValue<'v> for MetricValue {
    fn from_value(value: emit::Value<'v>) -> Option<Self> {
        if let Some(value) = value.to_i64() {
            return Some(MetricValue::I64(value));
        }

        let value = value.as_f64();
        if value.is_finite() {
            return Some(MetricValue::F64(value));
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
    use prost::Message;

    match body {
        Ok(body) => {
            let response =
                crate::data::generated::collector::metrics::v1::ExportMetricsServiceResponse::decode(
                    body,
                )
                .unwrap();

            emit::debug!(rt: emit::runtime::internal(), "received {#[emit::as_debug] response}");
        }
        Err(body) => {
            let response =
                crate::data::generated::collector::metrics::v1::ExportMetricsPartialSuccess::decode(body)
                    .unwrap();

            emit::warn!(rt: emit::runtime::internal(), "received {#[emit::as_debug] response}");
        }
    }
}
