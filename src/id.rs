use crate::empty::Empty;

#[derive(Clone, Copy)]
pub struct Id {
    trace: TraceId,
    span: SpanId,
}

#[derive(Clone, Copy)]
pub struct TraceId(u128);

#[derive(Clone, Copy)]
pub struct SpanId(u64);

impl Id {
    pub const EMPTY: Self = Id {
        trace: TraceId::EMPTY,
        span: SpanId::EMPTY,
    };

    pub fn new(trace: Option<TraceId>, span: Option<SpanId>) -> Self {
        Id {
            trace: trace.unwrap_or(TraceId::EMPTY),
            span: span.unwrap_or(SpanId::EMPTY),
        }
    }

    pub fn merge(&self, incoming: Id) -> Self {
        Id::new(
            // Use the trace id from the incoming, then our trace id, then try generate one
            // Ids are more likely to share the same trace id
            incoming
                .trace()
                .or(self.trace())
                .or_else(|| crate::ambient::get().trace()),
            // Use the span id from the incoming, then try generate one, then our span id
            // Ids are more likely to have unique span ids
            incoming
                .span()
                .or_else(|| crate::ambient::get().span())
                .or(self.span()),
        )
    }

    pub fn trace(&self) -> Option<TraceId> {
        if self.trace.is_empty() {
            None
        } else {
            Some(self.trace)
        }
    }

    pub fn span(&self) -> Option<SpanId> {
        if self.span.is_empty() {
            None
        } else {
            Some(self.span)
        }
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl From<SpanId> for Id {
    fn from(value: SpanId) -> Self {
        Id::new(None, Some(value))
    }
}

impl From<TraceId> for Id {
    fn from(value: TraceId) -> Self {
        Id::new(Some(value), None)
    }
}

const HEX: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

impl TraceId {
    const EMPTY: Self = TraceId(0);

    fn is_empty(&self) -> bool {
        self.0 == Self::EMPTY.0
    }

    pub fn from_u128(v: u128) -> Self {
        TraceId(v)
    }

    pub fn to_hex(&self) -> [u8; 32] {
        let mut dst = [0; 32];
        let src: [u8; 16] = self.0.to_ne_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX[(b & 0x0f) as usize];
        }

        dst
    }
}

impl SpanId {
    const EMPTY: Self = SpanId(0);

    fn is_empty(&self) -> bool {
        self.0 == Self::EMPTY.0
    }

    pub fn from_u64(v: u64) -> Self {
        SpanId(v)
    }

    pub fn to_hex(&self) -> [u8; 16] {
        let mut dst = [0; 16];
        let src: [u8; 8] = self.0.to_ne_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX[(b & 0x0f) as usize];
        }

        dst
    }
}

pub trait IdGenerator {
    fn trace(&self) -> Option<TraceId>;
    fn span(&self) -> Option<SpanId>;
}

impl<'a, T: IdGenerator + ?Sized> IdGenerator for &'a T {
    fn trace(&self) -> Option<TraceId> {
        (**self).trace()
    }

    fn span(&self) -> Option<SpanId> {
        (**self).span()
    }
}

impl<'a, T: IdGenerator> IdGenerator for Option<T> {
    fn trace(&self) -> Option<TraceId> {
        self.as_ref().and_then(|id| id.trace())
    }

    fn span(&self) -> Option<SpanId> {
        self.as_ref().and_then(|id| id.span())
    }
}

#[cfg(feature = "std")]
impl<'a, T: IdGenerator + ?Sized + 'a> IdGenerator for Box<T> {
    fn trace(&self) -> Option<TraceId> {
        (**self).trace()
    }

    fn span(&self) -> Option<SpanId> {
        (**self).span()
    }
}

impl IdGenerator for Id {
    fn trace(&self) -> Option<TraceId> {
        self.trace()
    }

    fn span(&self) -> Option<SpanId> {
        self.span()
    }
}

impl IdGenerator for Empty {
    fn trace(&self) -> Option<TraceId> {
        None
    }

    fn span(&self) -> Option<SpanId> {
        None
    }
}

mod internal {
    use super::{SpanId, TraceId};

    pub trait DispatchIdGenerator {
        fn dispatch_trace(&self) -> Option<TraceId>;
        fn dispatch_span(&self) -> Option<SpanId>;
    }

    pub trait SealedIdGenerator {
        fn erase_id_generator(&self) -> crate::internal::Erased<&dyn DispatchIdGenerator>;
    }
}

pub trait ErasedIdGenerator: internal::SealedIdGenerator {}

impl<T: IdGenerator> ErasedIdGenerator for T {}

impl<T: IdGenerator> internal::SealedIdGenerator for T {
    fn erase_id_generator(&self) -> crate::internal::Erased<&dyn internal::DispatchIdGenerator> {
        crate::internal::Erased(self)
    }
}

impl<T: IdGenerator> internal::DispatchIdGenerator for T {
    fn dispatch_trace(&self) -> Option<TraceId> {
        self.trace()
    }

    fn dispatch_span(&self) -> Option<SpanId> {
        self.span()
    }
}

impl<'a> IdGenerator for dyn ErasedIdGenerator + 'a {
    fn trace(&self) -> Option<TraceId> {
        self.erase_id_generator().0.dispatch_trace()
    }

    fn span(&self) -> Option<SpanId> {
        self.erase_id_generator().0.dispatch_span()
    }
}

impl<'a> IdGenerator for dyn ErasedIdGenerator + Send + Sync + 'a {
    fn trace(&self) -> Option<TraceId> {
        (self as &(dyn ErasedIdGenerator + 'a)).trace()
    }

    fn span(&self) -> Option<SpanId> {
        (self as &(dyn ErasedIdGenerator + 'a)).span()
    }
}

#[cfg(feature = "id-generator")]
mod rng_support {
    use super::*;

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
}

#[cfg(feature = "id-generator")]
pub use rng_support::*;
