use crate::{
    empty::Empty,
    value::{ToValue, Value},
};
use core::{
    fmt,
    num::{NonZeroU128, NonZeroU64},
    str,
    str::FromStr,
};

#[derive(Clone, Copy)]
pub struct TraceId(NonZeroU128);

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl FromStr for TraceId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl ToValue for TraceId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> Value<'v> {
    pub fn to_trace_id(&self) -> Option<TraceId> {
        self.downcast_ref::<TraceId>()
            .copied()
            .or_else(|| self.parse())
    }
}

impl TraceId {
    pub fn new(v: NonZeroU128) -> Self {
        TraceId(v)
    }

    pub fn from_u128(v: u128) -> Option<Self> {
        Some(TraceId(NonZeroU128::new(v)?))
    }

    pub fn to_u128(&self) -> u128 {
        self.0.get()
    }

    pub fn to_hex(&self) -> [u8; 32] {
        let mut dst = [0; 32];
        let src: [u8; 16] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX[(b & 0x0f) as usize];
        }

        dst
    }
}

#[derive(Clone, Copy)]
pub struct SpanId(NonZeroU64);

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl FromStr for SpanId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl ToValue for SpanId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> Value<'v> {
    pub fn to_span_id(&self) -> Option<SpanId> {
        self.downcast_ref::<SpanId>()
            .copied()
            .or_else(|| self.parse())
    }
}

impl SpanId {
    pub fn new(v: NonZeroU64) -> Self {
        SpanId(v)
    }

    pub fn from_u64(v: u64) -> Option<Self> {
        Some(SpanId(NonZeroU64::new(v)?))
    }

    pub fn to_u64(&self) -> u64 {
        self.0.get()
    }

    pub fn to_hex(&self) -> [u8; 16] {
        let mut dst = [0; 16];
        let src: [u8; 8] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX[(b & 0x0f) as usize];
        }

        dst
    }
}

const HEX: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

pub struct ParseIdError {}

pub trait IdGen {
    fn new_trace_id(&self) -> Option<TraceId>;
    fn new_span_id(&self) -> Option<SpanId>;
}

impl<'a, T: IdGen + ?Sized> IdGen for &'a T {
    fn new_trace_id(&self) -> Option<TraceId> {
        (**self).new_trace_id()
    }

    fn new_span_id(&self) -> Option<SpanId> {
        (**self).new_span_id()
    }
}

impl<'a, T: IdGen> IdGen for Option<T> {
    fn new_trace_id(&self) -> Option<TraceId> {
        self.as_ref().and_then(|id| id.new_trace_id())
    }

    fn new_span_id(&self) -> Option<SpanId> {
        self.as_ref().and_then(|id| id.new_span_id())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: IdGen + ?Sized + 'a> IdGen for alloc::boxed::Box<T> {
    fn new_trace_id(&self) -> Option<TraceId> {
        (**self).new_trace_id()
    }

    fn new_span_id(&self) -> Option<SpanId> {
        (**self).new_span_id()
    }
}

impl IdGen for Empty {
    fn new_trace_id(&self) -> Option<TraceId> {
        None
    }

    fn new_span_id(&self) -> Option<SpanId> {
        None
    }
}

mod internal {
    use super::{SpanId, TraceId};

    pub trait DispatchGenId {
        fn dispatch_new_trace_id(&self) -> Option<TraceId>;
        fn dispatch_new_span_id(&self) -> Option<SpanId>;
    }

    pub trait SealedIdGen {
        fn erase_id_gen(&self) -> crate::internal::Erased<&dyn DispatchGenId>;
    }
}

pub trait ErasedIdGen: internal::SealedIdGen {}

impl<T: IdGen> ErasedIdGen for T {}

impl<T: IdGen> internal::SealedIdGen for T {
    fn erase_id_gen(&self) -> crate::internal::Erased<&dyn internal::DispatchGenId> {
        crate::internal::Erased(self)
    }
}

impl<T: IdGen> internal::DispatchGenId for T {
    fn dispatch_new_trace_id(&self) -> Option<TraceId> {
        self.new_trace_id()
    }

    fn dispatch_new_span_id(&self) -> Option<SpanId> {
        self.new_span_id()
    }
}

impl<'a> IdGen for dyn ErasedIdGen + 'a {
    fn new_trace_id(&self) -> Option<TraceId> {
        self.erase_id_gen().0.dispatch_new_trace_id()
    }

    fn new_span_id(&self) -> Option<SpanId> {
        self.erase_id_gen().0.dispatch_new_span_id()
    }
}

impl<'a> IdGen for dyn ErasedIdGen + Send + Sync + 'a {
    fn new_trace_id(&self) -> Option<TraceId> {
        (self as &(dyn ErasedIdGen + 'a)).new_trace_id()
    }

    fn new_span_id(&self) -> Option<SpanId> {
        (self as &(dyn ErasedIdGen + 'a)).new_span_id()
    }
}
