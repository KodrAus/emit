#[cfg(feature = "std")]
use emit_core::{clock::ErasedClock, rng::ErasedRng, runtime::AssertInternal};

#[cfg(feature = "std")]
pub mod system_clock;

#[cfg(feature = "std")]
pub mod thread_local_ctxt;

#[cfg(feature = "rng")]
pub mod system_rng;

#[cfg(feature = "std")]
type DefaultClock = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultClock = emit_core::empty::Empty;

#[cfg(feature = "rng")]
type DefaultIdGen = system_rng::SystemRng;
#[cfg(not(feature = "rng"))]
type DefaultIdGen = emit_core::empty::Empty;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    pub(crate) clock: DefaultClock,
    #[cfg(feature = "std")]
    pub(crate) clock: AssertInternal<Box<dyn ErasedClock + Send + Sync>>,
    #[cfg(not(feature = "std"))]
    pub(crate) rng: DefaultIdGen,
    #[cfg(feature = "std")]
    pub(crate) rng: AssertInternal<Box<dyn ErasedRng + Send + Sync>>,
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
            clock: AssertInternal(Box::new(DefaultClock::default())),
            #[cfg(not(feature = "std"))]
            rng: DefaultIdGen::default(),
            #[cfg(feature = "std")]
            rng: AssertInternal(Box::new(DefaultIdGen::default())),
        }
    }
}
