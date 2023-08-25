use crate::{
    client::{OtlpClient, OtlpClientBuilder},
    proto::{
        collector::trace::v1::ExportTraceServiceRequest,
        common::v1::{any_value::Value, AnyValue, KeyValue},
        trace::v1::{span::Event, ResourceSpans, ScopeSpans, Span},
    },
    value::to_value,
};
use emit_batcher::BatchError;
use emit_core::well_known;
use std::{collections::HashSet, ops::ControlFlow, time::Duration};

pub fn http(dst: impl Into<String>) -> OtlpTraceEmitterBuilder {
    OtlpTraceEmitterBuilder {
        inner: OtlpClientBuilder::http(dst),
    }
}

pub struct OtlpTraceEmitterBuilder {
    inner: OtlpClientBuilder,
}

pub struct OtlpTraceEmitter {
    inner: OtlpClient<Span>,
}

impl OtlpTraceEmitterBuilder {
    pub fn resource(self, resource: impl emit_core::props::Props) -> Self {
        OtlpTraceEmitterBuilder {
            inner: self.inner.resource(resource),
        }
    }

    pub fn spawn(self) -> OtlpTraceEmitter {
        OtlpTraceEmitter {
            inner: self.inner.spawn(|client, batch| {
                client.emit(batch, |resource, scope, batch| {
                    use prost::Message;

                    let request = ExportTraceServiceRequest {
                        resource_spans: vec![ResourceSpans {
                            resource,
                            scope_spans: vec![ScopeSpans {
                                scope,
                                spans: batch.to_vec(),
                                schema_url: String::new(),
                            }],
                            schema_url: String::new(),
                        }],
                    };

                    let mut buf = Vec::new();
                    request.encode(&mut buf).map_err(BatchError::no_retry)?;

                    Ok(buf)
                })
            }),
        }
    }
}

impl emit_core::emitter::Emitter for OtlpTraceEmitter {
    fn emit<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        let (start_time_unix_nano, end_time_unix_nano) = evt
            .extent()
            .as_span()
            .map(|ts| {
                (
                    ts.start.as_unix_time().as_nanos() as u64,
                    ts.end.as_unix_time().as_nanos() as u64,
                )
            })
            .unwrap_or((0, 0));

        let name = evt.msg().to_string();

        let mut events = Vec::new();

        let mut attributes = Vec::new();
        let mut trace_id = Vec::new();
        let mut span_id = Vec::new();
        let mut parent_span_id = Vec::new();

        let mut seen = HashSet::new();
        evt.props().for_each(|k, v| {
            match k.as_str() {
                well_known::SPAN_ID_KEY => {
                    span_id = v
                        .to_span_id()
                        .map(|span_id| span_id.to_hex().to_vec())
                        .unwrap_or_default();
                }
                well_known::TRACE_ID_KEY => {
                    trace_id = v
                        .to_trace_id()
                        .map(|trace_id| trace_id.to_hex().to_vec())
                        .unwrap_or_default();
                }
                well_known::SPAN_PARENT_KEY => {
                    parent_span_id = v
                        .to_span_id()
                        .map(|parent_span_id| parent_span_id.to_hex().to_vec())
                        .unwrap_or_default();
                }
                well_known::ERR_KEY => {
                    let message = v.to_string();

                    events.push(Event {
                        time_unix_nano: end_time_unix_nano,
                        name: "exception".to_owned(),
                        attributes: vec![KeyValue {
                            key: "exception.message".to_owned(),
                            value: Some(AnyValue {
                                value: Some(Value::StringValue(message)),
                            }),
                        }],
                        dropped_attributes_count: 0,
                    });
                }
                _ => {
                    let key = k.to_string();

                    if seen.insert(k) {
                        let value = to_value(v);

                        attributes.push(KeyValue { key, value });
                    }
                }
            }

            ControlFlow::Continue(())
        });

        self.inner.emit(Span {
            start_time_unix_nano,
            end_time_unix_nano,
            attributes,
            dropped_attributes_count: 0,
            events,
            dropped_events_count: 0,
            links: Vec::new(),
            dropped_links_count: 0,
            trace_id,
            span_id,
            parent_span_id,
            trace_state: String::new(),
            name,
            kind: 0,
            status: None,
        })
    }

    fn blocking_flush(&self, timeout: Duration) {
        self.inner.blocking_flush(timeout)
    }
}
