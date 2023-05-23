use crate::id::{GenId, SpanId, TraceId};

#[derive(Default, Debug, Clone, Copy)]
pub struct RngGenId;

impl GenId for RngGenId {
    fn gen_trace(&self) -> Option<TraceId> {
        use rand::Rng;

        Some(TraceId::from_u128(rand::thread_rng().gen()))
    }

    fn gen_span(&self) -> Option<SpanId> {
        use rand::Rng;

        Some(SpanId::from_u64(rand::thread_rng().gen()))
    }
}
