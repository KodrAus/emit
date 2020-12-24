#![feature(once_cell)]

use std::{error::Error, fmt, lazy::SyncOnceCell};

use sval::value::{self, Value};

pub use emit_ct::{
    debug, emit, error, info, source, trace, warn, with_debug, with_display, with_serde, with_sval,
};

pub type Emitter = fn(&Record);

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
            .and_then(|source| source.to_error())
    }
}

#[doc(hidden)]
pub use emit_ct as ct;

#[doc(hidden)]
pub use emit_rt as rt;

#[doc(hidden)]
pub mod __private {
    use crate::{Emitter, Record};

    pub fn emit(record: &crate::rt::__private::Record) {
        crate::emit(&Record(record))
    }

    pub fn emit_to(target: Emitter, record: &crate::rt::__private::Record) {
        target(&Record(record))
    }
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
