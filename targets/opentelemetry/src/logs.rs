use crate::{
    id::{to_span_id, to_trace_id},
    key::to_key,
    value::to_any_value,
};
use emit_core::well_known::WellKnown;
use opentelemetry_api::{
    logs::{AnyValue, LogRecord, Logger as _, Severity},
    trace::{SpanContext, TraceFlags, TraceState},
    Key, OrderMap,
};
use opentelemetry_sdk::logs::Logger;
use std::{
    ops::ControlFlow,
    time::{Duration, SystemTime},
};

pub fn target(logger: Logger) -> OpenTelemetryLogsTarget {
    OpenTelemetryLogsTarget(logger)
}

pub struct OpenTelemetryLogsTarget(Logger);

impl emit_core::target::Target for OpenTelemetryLogsTarget {
    fn event<P: emit_core::props::Props>(&self, evt: &emit_core::event::Event<P>) {
        self.0.emit(to_record(evt));
    }

    // TODO: Respect the timeout
    fn blocking_flush(&self, _: Duration) {
        if let Some(provider) = self.0.provider() {
            let _ = provider.force_flush();
        }
    }
}

fn to_record(evt: &emit_core::event::Event<impl emit_core::props::Props>) -> LogRecord {
    let mut builder = LogRecord::builder();

    builder
        .with_span_context(&SpanContext::new(
            to_trace_id(evt.id()),
            to_span_id(evt.id()),
            TraceFlags::default(),
            false,
            TraceState::default(),
        ))
        .with_timestamp(
            evt.timestamp()
                .map(|ts| ts.to_system_time())
                .unwrap_or_else(SystemTime::now),
        )
        .with_severity_number(match evt.lvl().unwrap_or(emit_core::level::Level::Info) {
            emit_core::level::Level::Debug => Severity::Debug,
            emit_core::level::Level::Info => Severity::Info,
            emit_core::level::Level::Warn => Severity::Warn,
            emit_core::level::Level::Error => Severity::Error,
        })
        .with_severity_text(evt.lvl().to_string())
        .with_body(AnyValue::String(evt.msg().to_string().into()))
        .with_attributes({
            let mut attributes = OrderMap::<Key, AnyValue>::new();

            evt.props().for_each(|k, v| {
                if let Some(value) = to_any_value(v) {
                    attributes.insert(to_key(k), value);
                }

                ControlFlow::Continue(())
            });

            attributes
        })
        .build()
}
