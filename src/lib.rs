pub use emit_ct::*;

#[doc(hidden)]
pub use emit_rt::*;

use std::{fmt, mem, error::Error};

use sval::value::{self, Value};

use self::__private::TemplateRender;

pub type Emitter = fn(&Record);

pub fn target(emitter: Emitter) {
    let _ = __private::replace(unsafe { mem::transmute::<Emitter, __private::Emitter>(emitter) });
}

/**
An emitted record.
*/
#[repr(transparent)]
pub struct Record<'a>(__private::Record<'a>);

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
        self.0.template.render_kvs(self.0.kvs)
    }

    /**
    The original template associated with this record.
    */
    pub fn template<'b>(&'b self) -> impl fmt::Display + 'b {
        self.0.template.render_template()
    }

    /**
    The source error associated with this record.
    */
    pub fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.kvs.get("source").and_then(|source| source.to_error())
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
