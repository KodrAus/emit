use crate::id::{GenId, SpanId, TraceId};
use core::sync::atomic::{AtomicU64, Ordering};

// Value starts at 1; 0 is an empty span
static SPAN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Default, Debug, Clone, Copy)]
pub struct SeqGenId;

impl GenId for SeqGenId {
    fn gen_trace(&self) -> Option<TraceId> {
        // NOTE: Doesn't generate trace ids; these need to be globally unique
        None
    }

    fn gen_span(&self) -> Option<SpanId> {
        Some(SpanId::from_u64(SPAN_ID.fetch_add(1, Ordering::Relaxed)))
    }
}
