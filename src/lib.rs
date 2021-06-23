#![feature(once_cell)]

use std::{error::Error, fmt, lazy::SyncOnceCell};

use sval::value::{self, Value};

/**
Macros for emitting log events.
*/
pub use emit_ct::{
    debug, emit, error, info, source, trace, warn, as_debug, as_display, as_serde, as_sval,
};

/**
A type that receives and emits event records.
*/
pub type Emitter = fn(&Record);

/**
The global implicit emitter.
*/
static EMITTER: SyncOnceCell<Emitter> = SyncOnceCell::new();

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

impl<'a> Value for Record<'a> {
    fn stream(&self, stream: &mut value::Stream) -> value::Result {
        self.0.stream(stream)
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
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0
            .kvs
            .get("source")
            .and_then(|source| source.to_borrowed_error())
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

#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.pass("tests/ui/pass/*.rs");
        t.compile_fail("tests/ui/fail/*.rs");
    }
}
