use std::{borrow::Cow, cell::RefCell, ops::ControlFlow};

use emit::{str::ToStr, value::ToValue, well_known::KEY_SPAN_ID, Filter};
use opentelemetry::{
    global::{self, BoxedTracer, GlobalLoggerProvider},
    logs::{LogRecord, Logger, LoggerProvider},
    trace::{FutureExt, SpanId, TraceContextExt, TraceId, Tracer},
    Context, ContextGuard,
};

pub fn emitter(name: &'static str) -> OpenTelemetryEmitter {
    OpenTelemetryEmitter::new(name)
}

pub fn ctxt<C: emit::Ctxt>(name: &'static str, ctxt: C) -> OpenTelemetryContext<C> {
    OpenTelemetryContext::new(name, ctxt)
}

pub fn shutdown() {
    global::shutdown_logger_provider();
    global::shutdown_tracer_provider();
}

thread_local! {
    static CTXT_STACK: RefCell<Vec<CtxtFrame>> = RefCell::new(Vec::new());
}

struct CtxtFrame {
    guard: ContextGuard,
    ctxt: Context,
    open: bool,
}

fn push(guard: CtxtFrame) {
    CTXT_STACK.with(|stack| stack.borrow_mut().push(guard));
}

fn pop() -> Option<CtxtFrame> {
    CTXT_STACK.with(|stack| stack.borrow_mut().pop())
}

fn with_current(f: impl FnOnce(&mut CtxtFrame)) {
    CTXT_STACK.with(|stack| {
        if let Some(frame) = stack.borrow_mut().last_mut() {
            f(frame);
        }
    })
}

pub struct OpenTelemetryContext<C> {
    tracer: BoxedTracer,
    ctxt: C,
}

pub struct OpenTelemetryContextFrame<F> {
    ctxt: Option<Context>,
    attached: bool,
    open: bool,
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
            tracer: global::tracer(name),
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

        if let Some(span_id) = span_id {
            let span_id = otel_span_id(span_id);
            let trace_id = trace_id.map(otel_trace_id);

            let mut span = self.tracer.span_builder("emit_span").with_span_id(span_id);

            if let Some(trace_id) = trace_id {
                span = span.with_trace_id(trace_id);
            }

            let ctxt = Context::current_with_span(span.start(&self.tracer));

            OpenTelemetryContextFrame {
                attached: false,
                open: true,
                ctxt: Some(ctxt),
                frame: self.ctxt.open_root(props),
            }
        } else {
            OpenTelemetryContextFrame {
                attached: false,
                open: false,
                ctxt: None,
                frame: self.ctxt.open_root(props),
            }
        }
    }

    fn enter(&self, local: &mut Self::Frame) {
        self.ctxt.enter(&mut local.frame);

        if let Some(ctxt) = local.ctxt.take() {
            let guard = ctxt.attach();
            let ctxt = Context::current();

            push(CtxtFrame {
                guard,
                ctxt,
                open: true,
            });
            local.attached = true;
        }
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let ctxt = Context::current();
        let span = ctxt.span();

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
            local.ctxt = Some(Context::current());

            if let Some(frame) = pop() {
                local.open = frame.open;
            }
            local.attached = false;
        }
    }

    fn close(&self, mut local: Self::Frame) {
        if local.open {
            if let Some(ctxt) = local.ctxt.take() {
                let span = ctxt.span();
                span.end();
            }
        }
    }
}

pub struct OpenTelemetryEmitter {
    logger: <GlobalLoggerProvider as LoggerProvider>::Logger,
}

impl OpenTelemetryEmitter {
    pub fn new(name: &'static str) -> Self {
        OpenTelemetryEmitter {
            logger: global::logger_provider().logger(name),
        }
    }
}

impl emit::Emitter for OpenTelemetryEmitter {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        if emit::kind::is_span_filter().matches(evt) {
            let mut emitted = false;
            with_current(|frame| {
                if frame.open {
                    let span = frame.ctxt.span();

                    if Some(span.span_context().span_id())
                        == evt
                            .props()
                            .pull::<emit::span::SpanId, _>(KEY_SPAN_ID)
                            .map(otel_span_id)
                    {
                        // TODO: Add attributes

                        if let Some(name) = evt.props().get(emit::well_known::KEY_SPAN_NAME) {
                            span.update_name(name.to_string());
                        }

                        if let Some(extent) = evt.extent().and_then(|ex| ex.as_span()) {
                            span.end_with_timestamp(extent.end.to_system_time());
                        } else {
                            span.end();
                        }

                        frame.open = false;
                        emitted = true;
                    }
                }
            });

            if emitted {
                return;
            }
        }

        // TODO: Build a log record
        let record = LogRecord::builder()
            .with_body(evt.msg().to_string())
            .build();

        self.logger.emit(record);
    }

    fn blocking_flush(&self, _: std::time::Duration) {}
}

fn otel_trace_id(trace_id: emit::span::TraceId) -> TraceId {
    TraceId::from_bytes(trace_id.to_bytes())
}

fn otel_span_id(span_id: emit::span::SpanId) -> SpanId {
    SpanId::from_bytes(span_id.to_bytes())
}
