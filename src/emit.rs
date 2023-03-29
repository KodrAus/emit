#[cfg(feature = "std")]
use crate::std::{
    string::{String, ToString},
    sync::OnceLock,
};

use crate::Record;

/**
A type that receives and emits event records.
*/
pub type Emitter = fn(&Record);

/**
The global implicit emitter.
*/
#[cfg(feature = "std")]
static EMITTER: OnceLock<Emitter> = OnceLock::new();

/**
Set the default target to emit to.
*/
#[cfg(feature = "std")]
pub fn target(emitter: Emitter) {
    drop(EMITTER.set(emitter));
}

pub fn emit(record: &crate::rt::__private::Record) {
    #[cfg(feature = "std")]
    {
        if let Some(emitter) = EMITTER.get() {
            emitter(&Record(record))
        }
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = record;
    }
}

pub fn emit_to(target: Emitter, record: &crate::rt::__private::Record) {
    target(&Record(record))
}

#[cfg(feature = "std")]
pub fn format(record: &crate::rt::__private::Record) -> String {
    record.to_string()
}
