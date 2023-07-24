use emit_core::id::{IdSource, SpanId, TraceId};

#[derive(Default, Debug, Clone, Copy)]
pub struct Rng;

impl IdSource for Rng {
    fn trace_id(&self) -> Option<TraceId> {
        use rand::Rng;

        Some(TraceId::new(rand::thread_rng().gen()))
    }

    fn span_id(&self) -> Option<SpanId> {
        use rand::Rng;

        Some(SpanId::new(rand::thread_rng().gen()))
    }
}
