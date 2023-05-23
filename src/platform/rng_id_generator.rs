use crate::id::{IdGenerator, SpanId, TraceId};

#[derive(Default, Debug, Clone, Copy)]
pub struct RngIdGenerator;

impl IdGenerator for RngIdGenerator {
    fn trace(&self) -> Option<TraceId> {
        use rand::Rng;

        Some(TraceId::from_u128(rand::thread_rng().gen()))
    }

    fn span(&self) -> Option<SpanId> {
        use rand::Rng;

        Some(SpanId::from_u64(rand::thread_rng().gen()))
    }
}
