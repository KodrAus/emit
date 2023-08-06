use emit_core::id::{IdGen, SpanId, TraceId};

#[derive(Default, Debug, Clone, Copy)]
pub struct Rng;

impl IdGen for Rng {
    fn new_trace_id(&self) -> Option<TraceId> {
        use rand::Rng;

        Some(TraceId::new(rand::thread_rng().gen()))
    }

    fn new_span_id(&self) -> Option<SpanId> {
        use rand::Rng;

        Some(SpanId::new(rand::thread_rng().gen()))
    }
}
