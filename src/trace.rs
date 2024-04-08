use emit_core::{
    clock::Clock,
    extent::Extent,
    props::Props,
    rng::Rng,
    str::{Str, ToStr},
    value::FromValue,
    well_known::{EVENT_KIND_SPAN, KEY_EVENT_KIND, KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID},
};

use crate::{
    value::{ToValue, Value},
    Timer,
};
use core::{
    fmt,
    num::{NonZeroU128, NonZeroU64},
    ops::ControlFlow,
    str::{self, FromStr},
};

#[derive(Clone, Copy, PartialEq, Eq)]
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
        Self::try_from_hex_slice(s.as_bytes())
    }
}

impl ToValue for TraceId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for TraceId {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<TraceId>()
            .copied()
            .or_else(|| TraceId::try_from_hex(value).ok())
    }
}

impl TraceId {
    pub fn random<R: Rng>(rng: R) -> Option<Self> {
        let a = rng.gen_u64()? as u128;
        let b = (rng.gen_u64()? as u128) << 64;

        Some(TraceId::new(NonZeroU128::new(a | b)?))
    }

    pub const fn new(v: NonZeroU128) -> Self {
        TraceId(v)
    }

    pub fn from_u128(v: u128) -> Option<Self> {
        Some(TraceId(NonZeroU128::new(v)?))
    }

    pub const fn to_u128(&self) -> u128 {
        self.0.get()
    }

    pub fn from_bytes(v: [u8; 16]) -> Option<Self> {
        Self::from_u128(u128::from_be_bytes(v))
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        self.0.get().to_be_bytes()
    }

    pub fn to_hex(&self) -> [u8; 32] {
        let mut dst = [0; 32];
        let src: [u8; 16] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX_ENCODE_TABLE[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX_ENCODE_TABLE[(b & 0x0f) as usize];
        }

        dst
    }

    pub fn try_from_hex_slice(hex: &[u8]) -> Result<Self, ParseIdError> {
        let hex: &[u8; 32] = hex.try_into().map_err(|_| ParseIdError {})?;

        let mut dst = [0; 16];

        let mut i = 0;
        while i < 16 {
            // Convert a two-char hex value (like `A8`)
            // into a byte (like `10101000`)
            let h1 = HEX_DECODE_TABLE[hex[i * 2] as usize];
            let h2 = HEX_DECODE_TABLE[hex[i * 2 + 1] as usize];

            // We use `0xff` as a sentinel value to indicate
            // an invalid hex character sequence (like the letter `G`)
            if h1 | h2 == 0xff {
                return Err(ParseIdError {});
            }

            // The upper nibble needs to be shifted into position
            // to produce the final byte value
            dst[i] = SHL4_TABLE[h1 as usize] | h2;
            i += 1;
        }

        Ok(TraceId::new(
            NonZeroU128::new(u128::from_be_bytes(dst)).ok_or_else(|| ParseIdError {})?,
        ))
    }

    pub fn try_from_hex(hex: impl fmt::Display) -> Result<Self, ParseIdError> {
        let mut buf = Buffer::<32>::new();

        Self::try_from_hex_slice(buf.buffer(hex)?)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
        Self::try_from_hex_slice(s.as_bytes())
    }
}

impl ToValue for SpanId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for SpanId {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<SpanId>()
            .copied()
            .or_else(|| SpanId::try_from_hex(value).ok())
    }
}

impl SpanId {
    pub fn random<R: Rng>(rng: R) -> Option<Self> {
        let a = rng.gen_u64()?;

        Some(SpanId::new(NonZeroU64::new(a)?))
    }

    pub const fn new(v: NonZeroU64) -> Self {
        SpanId(v)
    }

    pub fn from_u64(v: u64) -> Option<Self> {
        Some(SpanId(NonZeroU64::new(v)?))
    }

    pub const fn to_u64(&self) -> u64 {
        self.0.get()
    }

    pub fn from_bytes(v: [u8; 8]) -> Option<Self> {
        Self::from_u64(u64::from_be_bytes(v))
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        self.0.get().to_be_bytes()
    }

    pub fn to_hex(&self) -> [u8; 16] {
        let mut dst = [0; 16];
        let src: [u8; 8] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX_ENCODE_TABLE[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX_ENCODE_TABLE[(b & 0x0f) as usize];
        }

        dst
    }

    pub fn try_from_hex_slice(hex: &[u8]) -> Result<Self, ParseIdError> {
        let hex: &[u8; 16] = hex.try_into().map_err(|_| ParseIdError {})?;

        let mut dst = [0; 8];

        let mut i = 0;
        while i < 8 {
            // Convert a two-char hex value (like `A8`)
            // into a byte (like `10101000`)
            let h1 = HEX_DECODE_TABLE[hex[i * 2] as usize];
            let h2 = HEX_DECODE_TABLE[hex[i * 2 + 1] as usize];

            // We use `0xff` as a sentinel value to indicate
            // an invalid hex character sequence (like the letter `G`)
            if h1 | h2 == 0xff {
                return Err(ParseIdError {});
            }

            // The upper nibble needs to be shifted into position
            // to produce the final byte value
            dst[i] = SHL4_TABLE[h1 as usize] | h2;
            i += 1;
        }

        Ok(SpanId::new(
            NonZeroU64::new(u64::from_be_bytes(dst)).ok_or_else(|| ParseIdError {})?,
        ))
    }

    pub fn try_from_hex(hex: impl fmt::Display) -> Result<Self, ParseIdError> {
        let mut buf = Buffer::<16>::new();

        Self::try_from_hex_slice(buf.buffer(hex)?)
    }
}

/*
Original implementation: https://github.com/uuid-rs/uuid/blob/main/src/parser.rs

Licensed under Apache 2.0
*/

const HEX_ENCODE_TABLE: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

const HEX_DECODE_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = match i {
            b'0'..=b'9' => i - b'0',
            b'a'..=b'f' => i - b'a' + 10,
            b'A'..=b'F' => i - b'A' + 10,
            _ => 0xff,
        };

        if i == 255 {
            break buf;
        }

        i += 1
    }
};

const SHL4_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = i.wrapping_shl(4);

        if i == 255 {
            break buf;
        }

        i += 1;
    }
};

#[derive(Debug)]
pub struct ParseIdError {}

struct Buffer<const N: usize> {
    hex: [u8; N],
    idx: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Buffer {
            hex: [0; N],
            idx: 0,
        }
    }

    fn buffer(&mut self, hex: impl fmt::Display) -> Result<&[u8], ParseIdError> {
        use fmt::Write as _;

        self.idx = 0;

        write!(self, "{}", hex).map_err(|_| ParseIdError {})?;

        Ok(&self.hex[..self.idx])
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let s = s.as_bytes();
        let next_idx = self.idx + s.len();

        if next_idx <= self.hex.len() {
            self.hex[self.idx..next_idx].copy_from_slice(s);
            self.idx = next_idx;

            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

pub struct Span<C: Clock, P: Props, F: FnOnce(Option<Extent>, SpanProps<P>)> {
    value: Option<(Timer<C>, SpanProps<P>)>,
    on_drop: Option<F>,
}

pub struct SpanProps<P> {
    ctxt: SpanCtxt,
    props: P,
}

pub struct SpanCtxt {
    trace_id: TraceId,
    span_parent: Option<SpanId>,
    span_id: SpanId,
}

impl SpanCtxt {
    pub fn new(trace_id: TraceId, span_parent: Option<SpanId>, span_id: SpanId) -> Self {
        SpanCtxt {
            trace_id,
            span_parent,
            span_id,
        }
    }

    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    pub fn span_parent(&self) -> Option<&SpanId> {
        self.span_parent.as_ref()
    }

    pub fn span_id(&self) -> &SpanId {
        &self.span_id
    }
}

impl Props for SpanCtxt {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_TRACE_ID.to_str(), self.trace_id.to_value())?;
        for_each(KEY_SPAN_ID.to_str(), self.span_id.to_value())?;

        if let Some(ref span_parent) = self.span_parent {
            for_each(KEY_SPAN_PARENT.to_str(), span_parent.to_value())?;
        }

        ControlFlow::Continue(())
    }
}

impl<P> SpanProps<P> {
    pub fn new(trace_id: TraceId, span_parent: Option<SpanId>, span_id: SpanId, props: P) -> Self {
        SpanProps {
            ctxt: SpanCtxt::new(trace_id, span_parent, span_id),
            props,
        }
    }

    pub fn trace_id(&self) -> &TraceId {
        &self.ctxt.trace_id
    }

    pub fn span_parent(&self) -> Option<&SpanId> {
        self.ctxt.span_parent.as_ref()
    }

    pub fn span_id(&self) -> &SpanId {
        &self.ctxt.span_id
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn ctxt(&self) -> &SpanCtxt {
        &self.ctxt
    }
}

impl<P: Props> Props for SpanProps<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_EVENT_KIND.to_str(), EVENT_KIND_SPAN.to_value())?;

        self.ctxt().for_each(&mut for_each)?;
        self.props.for_each(for_each)
    }
}

impl<C: Clock, P: Props, F: FnOnce(Option<Extent>, SpanProps<P>)> Drop for Span<C, P, F> {
    fn drop(&mut self) {
        if let Some((timer, props)) = self.value.take() {
            if let Some(on_drop) = self.on_drop.take() {
                on_drop(timer.extent(), props)
            }
        }
    }
}

impl<C: Clock, P: Props, F: FnOnce(Option<Extent>, SpanProps<P>)> Span<C, P, F> {
    pub fn new(timer: Timer<C>, props: SpanProps<P>, default_complete: F) -> Self {
        Span {
            value: Some((timer, props)),
            on_drop: Some(default_complete),
        }
    }

    pub fn disabled() -> Self {
        Span {
            value: None,
            on_drop: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.value.is_some()
    }

    pub fn timer(&self) -> Option<&Timer<C>> {
        self.value.as_ref().map(|(timer, _)| timer)
    }

    pub fn span_props(&self) -> Option<&SpanProps<P>> {
        self.value.as_ref().map(|(_, props)| props)
    }

    pub fn props(&self) -> Option<&P> {
        self.span_props().map(|props| props.props())
    }

    pub fn trace_id(&self) -> Option<&TraceId> {
        self.span_props().map(|props| props.trace_id())
    }

    pub fn span_parent(&self) -> Option<&SpanId> {
        self.span_props().and_then(|props| props.span_parent())
    }

    pub fn span_id(&self) -> Option<&SpanId> {
        self.span_props().map(|props| props.span_id())
    }

    pub fn complete(mut self, complete: impl FnOnce(Option<Extent>, SpanProps<P>)) -> bool {
        if let Some((timer, props)) = self.value.take() {
            complete(timer.extent(), props);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_id_roundtrip() {
        let id = SpanId::new(NonZeroU64::new(u64::MAX / 2).unwrap());

        let fmt = id.to_string();

        let parsed: SpanId = fmt.parse().unwrap();

        assert_eq!(id, parsed, "{}", fmt);
    }

    #[test]
    fn trace_id_roundtrip() {
        let id = TraceId::new(NonZeroU128::new(u128::MAX / 2).unwrap());

        let fmt = id.to_string();

        let parsed: TraceId = fmt.parse().unwrap();

        assert_eq!(id, parsed, "{}", fmt);
    }
}
