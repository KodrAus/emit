/*!
Emit structured events for programs and people.

`emit` is a front-end for capturing diagnostic data in programs and emitting them to
some outside observer. You can either configure `tracing` or your own function as the destination
for events.
*/

#![feature(once_cell)]

use std::{fmt, sync::OnceLock};

#[cfg(feature = "std")]
use std::error::Error;

/**
Macros for emitting log events.
*/
pub use emit_ct::{
    as_debug, as_display, as_serde, as_sval, debug, emit, error, format, info, source, trace, warn,
};

/**
A type that receives and emits event records.
*/
pub type Emitter = fn(&Record);

/**
The global implicit emitter.
*/
static EMITTER: OnceLock<Emitter> = OnceLock::new();

fn emit(record: &Record) {
    if let Some(emitter) = EMITTER.get() {
        emitter(record)
    }
}

/**
Set the default target to emit to.
*/
pub fn target(emitter: Emitter) {
    drop(EMITTER.set(emitter));
}

/**
An emitted record.
*/
pub struct Record<'a>(&'a rt::__private::Record<'a>);

/**
An emitted value.
*/
pub struct Value<'a>(rt::__private::ValueBag<'a>);

impl<'a> Value<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }

    pub fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }

    pub fn to_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }

    pub fn to_f64(&self) -> Option<f64> {
        self.0.to_f64()
    }

    pub fn to_bool(&self) -> Option<bool> {
        self.0.to_bool()
    }

    pub fn to_str(&self) -> Option<&str> {
        self.0.to_borrowed_str()
    }
}

#[cfg(feature = "sval")]
impl<'a> sval_lib::value::Value for Record<'a> {
    fn stream(&self, stream: &mut sval_lib::value::Stream) -> sval_lib::value::Result {
        self.0.stream(stream)
    }
}

#[cfg(feature = "serde")]
impl<'a> serde_lib::Serialize for Record<'a> {
    fn serialize<S: serde_lib::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval_lib::value::Value for Value<'a> {
    fn stream(&self, stream: &mut sval_lib::value::Stream) -> sval_lib::value::Result {
        self.0.stream(stream)
    }
}

#[cfg(feature = "serde")]
impl<'a> serde_lib::Serialize for Value<'a> {
    fn serialize<S: serde_lib::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'a> Record<'a> {
    /**
    The formatted message associated with this record.
    */
    pub fn msg<'b>(&'b self) -> impl fmt::Display + 'b {
        self.0.render_msg()
    }

    /**
    The original template associated with this record.
    */
    pub fn template<'b>(&'b self) -> impl fmt::Display + 'b {
        self.0.render_template()
    }

    /**
    The source error associated with this record.
    */
    #[cfg(feature = "std")]
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0
            .kvs
            .get("source")
            .and_then(|source| source.to_borrowed_error())
    }

    /**
    Get a key-value.
    */
    pub fn get(&self, k: impl AsRef<str>) -> Option<Value> {
        self.0.kvs.get(k.as_ref()).map(|v| Value(v.by_ref()))
    }
}

/**
Private entrypoint for the `ct` crate.

Code generation expects to find items at `emit::ct::__private`, not `emit_ct::__private`.
*/
#[doc(hidden)]
pub use emit_ct as ct;

/**
Private entrypoint for the `rt` crate.

Code generation expects to find items at `emit::rt::__private`, not `emit_rt::__private`.
*/
#[doc(hidden)]
pub use emit_rt as rt;

mod emit;

/**
Private entrypoint for the `emit` crate.

Code generation expects to find items at `emit::__private`.
*/
#[doc(hidden)]
pub mod __private {
    pub use crate::emit::*;
}
