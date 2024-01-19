use crate::{clock::Clock, rng::Rng, Timestamp};

#[cfg(not(feature = "std"))]
use emit_core::empty::Empty;

use emit_core::runtime::{InternalClock, InternalRng};

#[cfg(feature = "std")]
pub(crate) mod system_clock;

#[cfg(feature = "std")]
pub(crate) mod thread_local_ctxt;

#[cfg(feature = "rng")]
pub(crate) mod thread_rng;

#[cfg(feature = "std")]
type DefaultClock = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultClock = Empty;

#[cfg(feature = "rng")]
type DefaultIdGen = thread_rng::ThreadRng;
#[cfg(not(feature = "rng"))]
type DefaultIdGen = Empty;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;
#[cfg(not(feature = "std"))]
pub(crate) type DefaultCtxt = Empty;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    pub(crate) clock: DefaultClock,
    #[cfg(feature = "std")]
    pub(crate) clock: Box<dyn InternalClock + Send + Sync>,
    #[cfg(not(feature = "std"))]
    pub(crate) id_gen: DefaultIdGen,
    #[cfg(feature = "std")]
    pub(crate) rng: Box<dyn InternalRng + Send + Sync>,
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
            rng: DefaultIdGen::default(),
            #[cfg(feature = "std")]
            rng: Box::new(DefaultIdGen::default()),
        }
    }
}

impl Clock for Platform {
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl Rng for Platform {
    fn gen_u64(&self) -> Option<u64> {
        self.rng.gen_u64()
    }
}
