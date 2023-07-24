use crate::{id::IdSource, time::Clock, Timestamp};

#[cfg(not(feature = "std"))]
use emit_core::empty::Empty;

#[cfg(feature = "std")]
use emit_core::{id::ErasedIdSource, time::ErasedClock};

#[cfg(feature = "std")]
pub(crate) mod system_clock;

#[cfg(feature = "std")]
pub(crate) mod thread_local_ctxt;

#[cfg(feature = "rng")]
pub(crate) mod rng;

#[cfg(feature = "std")]
type DefaultTime = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultTime = Empty;

#[cfg(feature = "rng")]
type DefaultIdSource = rng::Rng;
#[cfg(not(feature = "rng"))]
type DefaultGenId = Empty;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    pub(crate) clock: DefaultTime,
    #[cfg(feature = "std")]
    pub(crate) clock: Box<dyn ErasedClock + Send + Sync>,
    #[cfg(not(feature = "std"))]
    pub(crate) gen_id: DefaultIdSource,
    #[cfg(feature = "std")]
    pub(crate) id_src: Box<dyn ErasedIdSource + Send + Sync>,
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
            clock: DefaultTime::default(),
            #[cfg(feature = "std")]
            clock: Box::new(DefaultTime::default()),
            #[cfg(not(feature = "std"))]
            id_src: DefaultIdSource::default(),
            #[cfg(feature = "std")]
            id_src: Box::new(DefaultIdSource::default()),
        }
    }
}

impl Clock for Platform {
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl IdSource for Platform {
    fn trace_id(&self) -> Option<crate::id::TraceId> {
        self.id_src.trace_id()
    }

    fn span_id(&self) -> Option<crate::id::SpanId> {
        self.id_src.span_id()
    }
}
