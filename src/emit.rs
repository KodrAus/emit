#[cfg(feature = "std")]
use crate::{
    std::string::{String, ToString},
    Emitter, EMITTER,
};

use crate::Record;

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
