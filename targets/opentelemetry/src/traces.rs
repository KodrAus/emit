/*
use crate::{
    id::{from_trace_span_ids, to_span_id, to_trace_id},
    key::to_key,
    value::to_value,
};
use emit_core::{id::IdSource, props::Props, template::Template, time::Timestamp};
use opentelemetry_api::{
    trace::{TraceContextExt, Tracer},
    Context, ContextGuard, OrderMap,
};
use std::{ops::ControlFlow, time::SystemTime};

pub fn ctxt<T: Tracer>(tracer: T) -> OpenTelemetryTracesCtxt<T>
where
    T::Span: Send + Sync + 'static,
{
    OpenTelemetryTracesCtxt(tracer)
}

pub struct OpenTelemetryTracesCtxt<T>(T);

pub struct OpenTelemetrySpan {
    cx: Option<Context>,
    active: Option<ContextGuard>,
}

impl<T: Tracer> emit_core::ctxt::Ctxt for OpenTelemetryTracesCtxt<T>
where
    T::Span: Send + Sync + 'static,
{
    type CurrentProps = emit_core::empty::Empty;
    type LocalFrame = OpenTelemetrySpan;

    fn open<P: Props>(
        &self,
        ts: Option<Timestamp>,
        id: IdSource,
        tpl: Template,
        props: P,
    ) -> Self::LocalFrame {
        let span = self
            .0
            .span_builder(tpl.to_string())
            .with_start_time(
                ts.map(|ts| ts.to_system_time())
                    .unwrap_or_else(SystemTime::now),
            )
            .with_span_id(to_span_id(id))
            .with_trace_id(to_trace_id(id))
            .with_attributes_map({
                let mut attributes = OrderMap::new();

                props.for_each(|k, v| {
                    attributes.insert(to_key(k), to_value(v));

                    ControlFlow::Continue(())
                });

                attributes
            })
            .start(&self.0);

        let cx = Context::current().with_span(span);

        OpenTelemetrySpan {
            cx: Some(cx),
            active: None,
        }
    }

    fn enter(&self, span: &mut Self::LocalFrame) {
        if let Some(cx) = span.cx.take() {
            span.active = Some(cx.attach());
        }
    }

    fn with_current<F: FnOnce(IdSource, &Self::CurrentProps)>(&self, with: F) {
        let cx = Context::current();

        let id = from_trace_span_ids(
            cx.span().span_context().trace_id(),
            cx.span().span_context().span_id(),
        );

        with(id, &emit_core::empty::Empty)
    }

    fn exit(&self, span: &mut Self::LocalFrame) {
        span.cx = Some(Context::current());
        drop(span.active.take());
    }

    fn close(&self, ts: Option<Timestamp>, mut span: Self::LocalFrame) {
        if let Some(cx) = span.cx.take() {
            cx.span().end_with_timestamp(
                ts.map(|ts| ts.to_system_time())
                    .unwrap_or_else(SystemTime::now),
            );
        }
    }
}
*/
