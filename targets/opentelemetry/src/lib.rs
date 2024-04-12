use std::{cell::RefCell, ops::ControlFlow};

use emit::{str::ToStr, value::ToValue, Filter};
use opentelemetry::{
    global::{BoxedTracer, GlobalLoggerProvider},
    logs::{LogRecord, Logger, LoggerProvider},
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

pub struct OpenTelemetryContext<C> {
    tracer: BoxedTracer,
    ctxt: C,
}

pub struct OpenTelemetryContextFrame<F> {
    ctxt: Option<Context>,
    attached: bool,
    frame: F,
}

pub struct OpenTelemetryContextProps<P: ?Sized> {
    trace_id: Option<emit::span::TraceId>,
    span_id: Option<emit::span::SpanId>,
    props: *const P,
}

impl<C> OpenTelemetryContext<C> {
    pub fn new(name: &'static str, ctxt: C) -> Self {
        OpenTelemetryContext {
            tracer: opentelemetry::global::tracer(name),
            ctxt,
        }
    }
}

impl<P: emit::Props + ?Sized> emit::Props for OpenTelemetryContextProps<P> {
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

        unsafe { &*self.props }.for_each(|k, v| {
            if k != emit::well_known::KEY_TRACE_ID && k != emit::well_known::KEY_SPAN_ID {
                for_each(k, v)?;
            }

            ControlFlow::Continue(())
        })
    }
}

impl<C: emit::Ctxt> emit::Ctxt for OpenTelemetryContext<C> {
    type Current = OpenTelemetryContextProps<C::Current>;

    type Frame = OpenTelemetryContextFrame<C::Frame>;

    fn open_root<P: emit::Props>(&self, props: P) -> Self::Frame {
        let trace_id = props.pull::<emit::span::TraceId, _>(emit::well_known::KEY_TRACE_ID);
        let span_id = props.pull::<emit::span::SpanId, _>(emit::well_known::KEY_SPAN_ID);
        let span_name = props
            .pull::<emit::Str, _>(emit::well_known::KEY_SPAN_NAME)
            .map(|name| name.to_cow());

        if let (Some(trace_id), Some(span_id), Some(span_name)) = (trace_id, span_id, span_name) {
            let span = self
                .tracer
                .span_builder(span_name)
                .with_span_id(SpanId::from_bytes(span_id.to_bytes()))
                .with_trace_id(TraceId::from_bytes(trace_id.to_bytes()));

            let ctxt = Context::current_with_span(span.start(&self.tracer));

            OpenTelemetryContextFrame {
                attached: false,
                ctxt: Some(ctxt),
                frame: self.ctxt.open_root(props),
            }
        } else {
            OpenTelemetryContextFrame {
                attached: false,
                ctxt: None,
                frame: self.ctxt.open_root(props),
            }
        }
    }

    fn enter(&self, local: &mut Self::Frame) {
        self.ctxt.enter(&mut local.frame);

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

        self.ctxt.with_current(|props| {
            let props = OpenTelemetryContextProps {
                trace_id: emit::span::TraceId::from_bytes(trace_id),
                span_id: emit::span::SpanId::from_bytes(span_id),
                props: props as *const C::Current,
            };

            with(&props)
        })
    }

    fn exit(&self, local: &mut Self::Frame) {
        self.ctxt.exit(&mut local.frame);

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

pub struct OpenTelemetryEmitter {
    logger: <GlobalLoggerProvider as LoggerProvider>::Logger,
}

impl OpenTelemetryEmitter {
    pub fn new(name: &'static str) -> Self {
        OpenTelemetryEmitter {
            logger: opentelemetry::global::logger_provider().logger(name),
        }
    }
}

impl emit::Emitter for OpenTelemetryEmitter {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        if emit::kind::is_span_filter().matches(evt) {
            let ctxt = Context::current();
            let span = ctxt.span();

            // TODO: Add attributes

            if let Some(extent) = evt.extent().and_then(|ex| ex.as_span()) {
                span.end_with_timestamp(extent.end.to_system_time());
            } else {
                span.end();
            }

            return;
        }

        // TODO: Build a log record
        let record = LogRecord::builder()
            .with_body(evt.msg().to_string())
            .build();

        self.logger.emit(record);
    }

    fn blocking_flush(&self, _: std::time::Duration) {}
}
