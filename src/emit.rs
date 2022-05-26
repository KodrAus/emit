use crate::{Emitter, Record};

pub fn emit(record: &crate::rt::__private::Record) {
    crate::emit(&Record(record))
}

pub fn emit_to(target: Emitter, record: &crate::rt::__private::Record) {
    target(&Record(record))
}

pub fn format(record: &crate::rt::__private::Record) -> String {
    record.render_msg().to_string()
}
