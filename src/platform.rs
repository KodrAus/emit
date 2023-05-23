use crate::{id::GenId, time::Clock, Timestamp};

#[cfg(not(feature = "std"))]
use crate::empty::Empty;

#[cfg(feature = "std")]
use crate::{id::ErasedGenId, time::ErasedClock};

#[cfg(feature = "std")]
pub(crate) mod system_clock;

#[cfg(feature = "std")]
pub(crate) mod thread_local_ctxt;

#[cfg(feature = "id-generator")]
pub(crate) mod rng_gen_id;

#[cfg(feature = "std")]
type DefaultTime = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultTime = Empty;

#[cfg(not(feature = "id-generator"))]
type DefaultGenId = Empty;
#[cfg(feature = "id-generator")]
type DefaultGenId = rng_gen_id::RngGenId;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    clock: DefaultTime,
    #[cfg(feature = "std")]
    clock: Box<dyn ErasedClock + Send + Sync>,
    #[cfg(not(feature = "std"))]
    gen_id: DefaultGenId,
    #[cfg(feature = "std")]
    gen_id: Box<dyn ErasedGenId + Send + Sync>,
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform {
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        Platform {
            #[cfg(not(feature = "std"))]
            clock: DefaultTime::default(),
            #[cfg(feature = "std")]
            clock: Box::new(DefaultTime::default()),
            #[cfg(not(feature = "std"))]
            gen_id: DefaultGenId::default(),
            #[cfg(feature = "std")]
            gen_id: Box::new(DefaultGenId::default()),
        }
    }
}

impl Clock for Platform {
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl GenId for Platform {
    fn gen(&self) -> crate::Id {
        self.gen_id.gen()
    }

    fn gen_trace(&self) -> Option<crate::id::TraceId> {
        self.gen_id.gen_trace()
    }

    fn gen_span(&self) -> Option<crate::id::SpanId> {
        self.gen_id.gen_span()
    }
}
