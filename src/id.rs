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

    pub fn or_gen(&self, incoming: Id, generator: impl GenId) -> Self {
        Id::new(
            // Use the trace id from the incoming, then our trace id, then try generate one
            // Ids are more likely to share the same trace id
            self.trace()
                .or(incoming.trace())
                .or_else(|| generator.gen_trace()),
            // Use the span id from the incoming, then try generate one, then our span id
            // Ids are more likely to have unique span ids
            self.span()
                .or_else(|| generator.gen_span())
                .or(incoming.span()),
        )
    }

    pub fn or(&self, incoming: Id) -> Self {
        Id::new(
            self.trace().or(incoming.trace()),
            self.span().or(incoming.span()),
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

pub trait GenId {
    fn gen_id(&self) -> Id {
        Id::new(self.gen_trace(), self.gen_span())
    }

    fn gen_trace(&self) -> Option<TraceId>;
    fn gen_span(&self) -> Option<SpanId>;
}

impl<'a, T: GenId + ?Sized> GenId for &'a T {
    fn gen_id(&self) -> Id {
        (**self).gen_id()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        (**self).gen_trace()
    }

    fn gen_span(&self) -> Option<SpanId> {
        (**self).gen_span()
    }
}

impl<'a, T: GenId> GenId for Option<T> {
    fn gen_id(&self) -> Id {
        self.as_ref()
            .map(|id| Id::new(id.gen_trace(), id.gen_span()))
            .unwrap_or_default()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        self.as_ref().and_then(|id| id.gen_trace())
    }

    fn gen_span(&self) -> Option<SpanId> {
        self.as_ref().and_then(|id| id.gen_span())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: GenId + ?Sized + 'a> GenId for alloc::boxed::Box<T> {
    fn gen_id(&self) -> Id {
        (**self).gen_id()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        (**self).gen_trace()
    }

    fn gen_span(&self) -> Option<SpanId> {
        (**self).gen_span()
    }
}

impl GenId for Empty {
    fn gen_id(&self) -> Id {
        Default::default()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        None
    }

    fn gen_span(&self) -> Option<SpanId> {
        None
    }
}

mod internal {
    use super::{Id, SpanId, TraceId};

    pub trait DispatchGenId {
        fn dispatch_gen_id(&self) -> Id;
        fn dispatch_gen_trace(&self) -> Option<TraceId>;
        fn dispatch_gen_span(&self) -> Option<SpanId>;
    }

    pub trait SealedIdGenerator {
        fn erase_gen_id(&self) -> crate::internal::Erased<&dyn DispatchGenId>;
    }
}

pub trait ErasedGenId: internal::SealedIdGenerator {}

impl<T: GenId> ErasedGenId for T {}

impl<T: GenId> internal::SealedIdGenerator for T {
    fn erase_gen_id(&self) -> crate::internal::Erased<&dyn internal::DispatchGenId> {
        crate::internal::Erased(self)
    }
}

impl<T: GenId> internal::DispatchGenId for T {
    fn dispatch_gen_id(&self) -> Id {
        self.gen_id()
    }

    fn dispatch_gen_trace(&self) -> Option<TraceId> {
        self.gen_trace()
    }

    fn dispatch_gen_span(&self) -> Option<SpanId> {
        self.gen_span()
    }
}

impl<'a> GenId for dyn ErasedGenId + 'a {
    fn gen_id(&self) -> Id {
        self.erase_gen_id().0.dispatch_gen_id()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        self.erase_gen_id().0.dispatch_gen_trace()
    }

    fn gen_span(&self) -> Option<SpanId> {
        self.erase_gen_id().0.dispatch_gen_span()
    }
}

impl<'a> GenId for dyn ErasedGenId + Send + Sync + 'a {
    fn gen_id(&self) -> Id {
        (self as &(dyn ErasedGenId + 'a)).gen_id()
    }

    fn gen_trace(&self) -> Option<TraceId> {
        (self as &(dyn ErasedGenId + 'a)).gen_trace()
    }

    fn gen_span(&self) -> Option<SpanId> {
        (self as &(dyn ErasedGenId + 'a)).gen_span()
    }
}
