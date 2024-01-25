pub struct TracingCtxt;

pub struct TracingCurrent;

pub struct TracingFrame(Option<tracing::Id>);

impl emit::Ctxt for TracingCtxt {
    type Current = TracingCurrent;

    type Frame = TracingFrame;

    fn open<P: emit::Props>(&self, props: P) -> Self::Frame {
        static METADATA: tracing::Metadata = tracing::Metadata::new(
            "emit_tracing",
            "emit_tracing",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], CALLSITE.interest()),
            tracing::metadata::Kind::HINT,
        );

        static CALLSITE: tracing::callsite::DefaultCallsite =
            tracing::callsite::DefaultCallsite::new(&METADATA);

        if props.pull::<emit::SpanId>().is_some() {
            let id = tracing::dispatcher::get_default(|dispatcher| {
                dispatcher.new_span(&tracing::span::Attributes::new(&METADATA, &[]))
            });

            TracingFrame(Some(id))
        } else {
            TracingFrame(None)
        }
    }

    fn enter(&self, local: &mut Self::Frame) {
        if let Some(ref id) = local.0 {
            tracing::dispatcher::get_default(|dispatcher| {
                dispatcher.enter(id);
            })
        }
    }

    fn with_current<F: FnOnce(&Self::Current)>(&self, with: F) {
        if let Some(ref id) = local.0 {
            tracing::dispatcher::get_default(|dispatcher| {})
        }
    }

    fn exit(&self, local: &mut Self::Frame) {
        if let Some(ref id) = local.0 {
            tracing::dispatcher::get_default(|dispatcher| dispatcher.exit(&local.0))
        }
    }

    fn close(&self, frame: Self::Frame) {
        if let Some(id) = local.0 {
            let _ = tracing::dispatcher::get_default(|dispatcher| dispatcher.try_close(frame.0));
        }
    }
}
