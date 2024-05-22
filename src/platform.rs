/*!
Components provided by the underlying platform.

This module defines implementations of [`crate::runtime::Runtime`] components that use capabilities of the host platform.
*/

#[cfg(feature = "std")]
use emit_core::{clock::ErasedClock, rng::ErasedRng, runtime::AssertInternal};

#[cfg(feature = "std")]
pub mod system_clock;

#[cfg(feature = "std")]
pub mod thread_local_ctxt;

#[cfg(feature = "rng")]
pub mod rand_rng;

#[cfg(feature = "std")]
type DefaultClock = system_clock::SystemClock;

#[cfg(feature = "rng")]
type DefaultIdGen = rand_rng::RandRng;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

/**
A type-erased container for system services used when intiailizing runtimes.
*/
pub(crate) struct Platform {
    #[cfg(feature = "std")]
    pub(crate) clock: AssertInternal<Box<dyn ErasedClock + Send + Sync>>,
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
            #[cfg(feature = "std")]
            clock: AssertInternal(Box::new(DefaultClock::default())),
            #[cfg(feature = "std")]
            rng: AssertInternal(Box::new(DefaultIdGen::default())),
        }
    }
}
