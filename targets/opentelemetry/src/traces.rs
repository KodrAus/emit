use crate::id::{from_trace_span_ids, to_span_id, to_trace_id};
use crate::key::to_key;
use crate::value::to_value;
use emit_core::id::Id;
use emit_core::props::Props;
use emit_core::template::Template;
use opentelemetry_api::trace::{TraceContextExt, Tracer};
use opentelemetry_api::{Context, ContextGuard, OrderMap};
use std::ops::ControlFlow;

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
    type Props = emit_core::empty::Empty;
    type Span = OpenTelemetrySpan;

    fn open<P: Props>(&self, id: Id, tpl: Template, props: P) -> Self::Span {
        let span = self
            .0
            .span_builder(tpl.to_string())
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

    fn enter(&self, span: &mut Self::Span) {
        if let Some(cx) = span.cx.take() {
            span.active = Some(cx.attach());
        }
    }

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        let cx = Context::current();

        let id = from_trace_span_ids(
            cx.span().span_context().trace_id(),
            cx.span().span_context().span_id(),
        );

        with(id, &emit_core::empty::Empty)
    }

    fn exit(&self, span: &mut Self::Span) {
        span.cx = Some(Context::current());
        drop(span.active.take());
    }

    fn close(&self, mut span: Self::Span) {
        if let Some(cx) = span.cx.take() {
            cx.span().end();
        }
    }
}
