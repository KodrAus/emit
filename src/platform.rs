use crate::{clock::Clock, id::IdGen, Timestamp};

#[cfg(not(feature = "std"))]
use emit_core::empty::Empty;

#[cfg(feature = "std")]
use emit_core::{clock::ErasedClock, id::ErasedIdGen};

#[cfg(feature = "std")]
pub(crate) mod system_clock;

#[cfg(feature = "std")]
pub(crate) mod thread_local_ctxt;

#[cfg(feature = "rng")]
pub(crate) mod rng;

#[cfg(feature = "std")]
type DefaultClock = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultClock = Empty;

#[cfg(feature = "rng")]
type DefaultIdGen = rng::Rng;
#[cfg(not(feature = "rng"))]
type DefaultIdGen = Empty;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    pub(crate) clock: DefaultClock,
    #[cfg(feature = "std")]
    pub(crate) clock: Box<dyn ErasedClock + Send + Sync>,
    #[cfg(not(feature = "std"))]
    pub(crate) id_gen: DefaultIdGen,
    #[cfg(feature = "std")]
    pub(crate) id_gen: Box<dyn ErasedIdGen + Send + Sync>,
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform {
    pub fn new() -> Self {
        Platform {
            #[cfg(not(feature = "std"))]
            clock: DefaultClock::default(),
            #[cfg(feature = "std")]
            clock: Box::new(DefaultClock::default()),
            #[cfg(not(feature = "std"))]
            id_gen: DefaultIdGen::default(),
            #[cfg(feature = "std")]
            id_gen: Box::new(DefaultIdGen::default()),
        }
    }
}

impl Clock for Platform {
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl IdGen for Platform {
    fn new_trace_id(&self) -> Option<crate::id::TraceId> {
        self.id_gen.new_trace_id()
    }

    fn new_span_id(&self) -> Option<crate::id::SpanId> {
        self.id_gen.new_span_id()
    }
}
