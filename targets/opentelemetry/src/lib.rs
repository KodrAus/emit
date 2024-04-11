use std::{cell::RefCell, ops::ControlFlow};

use emit::{str::ToStr, value::ToValue};
use opentelemetry::{
    trace::{SpanId, TraceContextExt, TraceId, Tracer},
    Context, ContextGuard,
};

thread_local! {
    static ACTIVE_GUARD: RefCell<Vec<ContextGuard>> = RefCell::new(Vec::new());
}

fn push_frame_guard(guard: ContextGuard) {
    ACTIVE_GUARD.with(|stack| stack.borrow_mut().push(guard));
}

fn pop_frame_guard() -> Option<ContextGuard> {
    ACTIVE_GUARD.with(|stack| stack.borrow_mut().pop())
}

pub struct OpenTelemetryContext(opentelemetry::global::BoxedTracer);

pub struct OpenTelemetryEmitter;

pub struct OpenTelemetryContextFrame {
    ctxt: Option<Context>,
    attached: bool,
}

pub struct OpenTelemetryContextProps {
    trace_id: Option<emit::span::TraceId>,
    span_id: Option<emit::span::SpanId>,
}

impl emit::Props for OpenTelemetryContextProps {
    fn for_each<'kv, F: FnMut(emit::Str<'kv>, emit::Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref trace_id) = self.trace_id {
            for_each(emit::well_known::KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }

        if let Some(ref span_id) = self.span_id {
            for_each(emit::well_known::KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }

        ControlFlow::Continue(())
    }
}

/*
Just make sure we do trace/span ids correctly here.

All other context should flow through. This way, when we use emit to wrap up events,
you still get trace and span ids propagating in other areas.
*/
impl emit::Ctxt for OpenTelemetryContext {
    type Current = OpenTelemetryContextProps;

    type Frame = OpenTelemetryContextFrame;

    fn open_root<P: emit::Props>(&self, props: P) -> Self::Frame {
        let trace_id = props.pull::<emit::span::TraceId, _>(emit::well_known::KEY_TRACE_ID);
        let span_id = props.pull::<emit::span::SpanId, _>(emit::well_known::KEY_SPAN_ID);

        if let Some(span_id) = span_id {
            let mut span = self
                .0
                .span_builder("emit")
                .with_span_id(SpanId::from_bytes(span_id.to_bytes()));

            if let Some(trace_id) = trace_id {
                span = span.with_trace_id(TraceId::from_bytes(trace_id.to_bytes()));
            }

            let ctxt = Context::current_with_span(span.start(&self.0));

            OpenTelemetryContextFrame {
                attached: false,
                ctxt: Some(ctxt),
            }
        } else {
            OpenTelemetryContextFrame {
                attached: false,
                ctxt: None,
            }
        }
    }

    fn enter(&self, local: &mut Self::Frame) {
        if let Some(ctxt) = local.ctxt.take() {
            push_frame_guard(ctxt.attach());
            local.attached = true;
        }
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let current = Context::current();

        let span = current.span();

        let trace_id = span.span_context().trace_id().to_bytes();
        let span_id = span.span_context().span_id().to_bytes();

        let props = OpenTelemetryContextProps {
            trace_id: emit::span::TraceId::from_bytes(trace_id),
            span_id: emit::span::SpanId::from_bytes(span_id),
        };

        with(&props)
    }

    fn exit(&self, local: &mut Self::Frame) {
        if local.attached {
            local.attached = false;

            local.ctxt = Some(Context::current());
            pop_frame_guard();
        }
    }

    fn close(&self, local: Self::Frame) {
        let _ = local;
    }
}

impl emit::Emitter for OpenTelemetryEmitter {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        /*
        We need some way to tell spans created by emit from those created by other libraries.

        Basically, we have a problem where we can't distinguish a "span" from some other kind of event.
        */
        if let Some(extent) = evt.extent().and_then(|ex| ex.as_span()) {
            let ctxt = Context::current();
            let span = ctxt.span();

            span.end_with_timestamp(extent.end.to_system_time());
        }
    }

    fn blocking_flush(&self, timeout: std::time::Duration) {
        todo!()
    }
}
