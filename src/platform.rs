use crate::{id::IdGenerator, time::Time, Timestamp};

#[cfg(not(feature = "std"))]
use crate::empty::Empty;

#[cfg(feature = "std")]
use crate::{id::ErasedIdGenerator, time::ErasedTime};

#[cfg(feature = "std")]
pub(crate) mod system_clock;

#[cfg(feature = "std")]
pub(crate) mod thread_local_ctxt;

#[cfg(feature = "id-generator")]
pub(crate) mod rng_id_generator;

#[cfg(feature = "std")]
type DefaultTime = system_clock::SystemClock;
#[cfg(not(feature = "std"))]
type DefaultTime = Empty;

#[cfg(not(feature = "id-generator"))]
type DefaultIdGenerator = Empty;
#[cfg(feature = "id-generator")]
type DefaultIdGenerator = rng_id_generator::RngIdGenerator;

#[cfg(feature = "std")]
pub(crate) type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

pub(crate) struct Platform {
    #[cfg(not(feature = "std"))]
    time: DefaultTime,
    #[cfg(feature = "std")]
    time: Box<dyn ErasedTime + Send + Sync>,
    #[cfg(not(feature = "std"))]
    id_generator: DefaultIdGenerator,
    #[cfg(feature = "std")]
    id_generator: Box<dyn ErasedIdGenerator + Send + Sync>,
}

impl Platform {
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        Platform {
            #[cfg(not(feature = "std"))]
            time: DefaultTime::default(),
            #[cfg(feature = "std")]
            time: Box::new(DefaultTime::default()),
            #[cfg(not(feature = "std"))]
            id_generator: DefaultIdGenerator::default(),
            #[cfg(feature = "std")]
            id_generator: Box::new(DefaultIdGenerator::default()),
        }
    }
}

impl Time for Platform {
    fn timestamp(&self) -> Option<Timestamp> {
        self.time.timestamp()
    }
}

impl IdGenerator for Platform {
    fn trace(&self) -> Option<crate::id::TraceId> {
        self.id_generator.trace()
    }

    fn span(&self) -> Option<crate::id::SpanId> {
        self.id_generator.span()
    }
}
