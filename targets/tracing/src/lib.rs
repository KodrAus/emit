#![no_std]

use core::{fmt, ops::ControlFlow, time::Duration};

use emit::well_known::{SPAN_ID_KEY, TRACE_ID_KEY};

pub fn ctxt<C: emit::Ctxt, S: tracing::Subscriber>(
    emit_ctxt: C,
    tracing_subscriber: S,
) -> TracingCtxt<C, S> {
    TracingCtxt(emit_ctxt, tracing_subscriber)
}

pub fn emitter<S: tracing::Subscriber>(tracing_subscriber: S) -> TracingEmitter<S> {
    TracingEmitter(tracing_subscriber)
}

pub struct TracingCtxt<C, S>(C, S);

pub struct TracingFrame<F>(Option<tracing::Id>, F);

impl<C: emit::Ctxt, S: tracing::Subscriber> emit::Ctxt for TracingCtxt<C, S> {
    type Current = C::Current;

    type Frame = TracingFrame<C::Frame>;

    fn open_root<P: emit::Props>(&self, props: P) -> Self::Frame {
        static METADATA: tracing::Metadata = tracing::Metadata::new(
            "emit_tracing::span",
            "emit_tracing::span",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(
                &[
                    emit::well_known::TRACE_ID_KEY,
                    emit::well_known::SPAN_ID_KEY,
                ],
                tracing_core::identify_callsite!(&CALLSITE),
            ),
            tracing::metadata::Kind::HINT,
        );

        static CALLSITE: tracing::callsite::DefaultCallsite =
            tracing::callsite::DefaultCallsite::new(&METADATA);

        let tracing_id = if let Some(span_id) = props.pull::<_, emit::SpanId>(SPAN_ID_KEY) {
            let fields = tracing::field::FieldSet::new(
                &[
                    emit::well_known::TRACE_ID_KEY,
                    emit::well_known::SPAN_ID_KEY,
                ],
                tracing_core::identify_callsite!(&CALLSITE),
            );

            let trace_id = props
                .pull::<_, emit::TraceId>(TRACE_ID_KEY)
                .map(tracing::field::display);

            let id = self.1.new_span(&tracing::span::Attributes::new(
                &METADATA,
                &fields.value_set(&[
                    (
                        &fields.field(emit::well_known::TRACE_ID_KEY).unwrap(),
                        trace_id
                            .as_ref()
                            .map(|trace_id| trace_id as &dyn tracing::Value),
                    ),
                    (
                        &fields.field(emit::well_known::SPAN_ID_KEY).unwrap(),
                        Some(&tracing::field::display(span_id) as &dyn tracing::Value),
                    ),
                ]),
            ));

            Some(id)
        } else {
            None
        };

        TracingFrame(tracing_id, self.0.open_root(props))
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if let Some(ref id) = frame.0 {
            self.1.enter(id);
        };

        self.0.enter(&mut frame.1)
    }

    fn with_current<F: FnOnce(&Self::Current)>(&self, with: F) {
        self.0.with_current(with)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if let Some(ref id) = frame.0 {
            self.1.exit(id);
        }

        self.0.exit(&mut frame.1)
    }

    fn close(&self, frame: Self::Frame) {
        if let Some(id) = frame.0 {
            let _ = self.1.try_close(id);
        }

        self.0.close(frame.1)
    }
}

pub struct TracingEmitter<S>(S);

impl<S: tracing::Subscriber> emit::Emitter for TracingEmitter<S> {
    fn emit<P: emit::Props>(&self, evt: &emit::Event<P>) {
        use emit::Props as _;

        static METADATA: tracing::Metadata = tracing::Metadata::new(
            "emit_tracing::event",
            "emit_tracing::event",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(
                &[
                    emit::well_known::TS_KEY,
                    emit::well_known::TS_START_KEY,
                    emit::well_known::MSG_KEY,
                    emit::well_known::TPL_KEY,
                    "props",
                ],
                tracing_core::identify_callsite!(&CALLSITE),
            ),
            tracing::metadata::Kind::HINT,
        );

        static CALLSITE: tracing::callsite::DefaultCallsite =
            tracing::callsite::DefaultCallsite::new(&METADATA);

        let fields = tracing::field::FieldSet::new(
            &[
                emit::well_known::TS_KEY,
                emit::well_known::TS_START_KEY,
                emit::well_known::MSG_KEY,
                emit::well_known::TPL_KEY,
                "props",
            ],
            tracing_core::identify_callsite!(&CALLSITE),
        );

        let mut ts = None;
        let mut ts_start = None;

        if let Some(extent) = evt.extent() {
            let range = extent.as_range();

            ts = Some(tracing::field::display(range.end));

            if extent.is_span() {
                ts_start = Some(tracing::field::display(range.start));
            }
        }

        let msg = tracing::field::display(evt.msg());
        let tpl = tracing::field::display(evt.tpl());
        let props = tracing::field::debug(DebugProps(evt.props().filter(|k, _| {
            k != emit::well_known::TRACE_ID_KEY && k != emit::well_known::SPAN_ID_KEY
        })));

        self.0.event(&tracing::Event::new(
            &METADATA,
            &fields.value_set(&[
                (
                    &fields.field(emit::well_known::TS_KEY).unwrap(),
                    ts.as_ref().map(|ts| ts as &dyn tracing::Value),
                ),
                (
                    &fields.field(emit::well_known::TS_START_KEY).unwrap(),
                    ts_start
                        .as_ref()
                        .map(|ts_start| ts_start as &dyn tracing::Value),
                ),
                (
                    &fields.field(emit::well_known::MSG_KEY).unwrap(),
                    Some(&msg as &dyn tracing::Value),
                ),
                (
                    &fields.field(emit::well_known::TPL_KEY).unwrap(),
                    Some(&tpl as &dyn tracing::Value),
                ),
                (
                    &fields.field("props").unwrap(),
                    Some(&props as &dyn tracing::Value),
                ),
            ]),
        ))
    }

    fn blocking_flush(&self, timeout: Duration) {}
}

struct DebugProps<P>(P);

impl<P: emit::Props> fmt::Debug for DebugProps<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();

        self.0.for_each(|k, v| {
            let _ = map.entry(&k, &v);
            ControlFlow::Continue(())
        });

        map.finish()
    }
}