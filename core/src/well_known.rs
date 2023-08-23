use crate::{
    id::{SpanId, TraceId},
    level::Level,
    props::Props,
    time::Timestamp,
    value::Value,
};

pub const TIMESTAMP_KEY: &'static str = "#ts";
pub const TIMESTAMP_START_KEY: &'static str = "#tss";
pub const MSG_KEY: &'static str = "#msg";
pub const TPL_KEY: &'static str = "#tpl";

pub const ERR_KEY: &'static str = "err";
pub const LVL_KEY: &'static str = "lvl";
pub const MODULE_KEY: &'static str = "mod";
pub const TRACE_ID_KEY: &'static str = "trace_id";
pub const SPAN_ID_KEY: &'static str = "span_id";
pub const SPAN_PARENT_KEY: &'static str = "span_parent";

pub const fn is_reserved(key: &str) -> bool {
    let key = key.as_bytes();

    if key.len() > 1 {
        key[0] == b'#' && key[1] != b'#'
    } else if key.len() == 1 {
        key[0] == b'#'
    } else {
        false
    }
}

pub trait WellKnown: Props {
    fn timestamp(&self) -> Option<Timestamp> {
        self.get(TIMESTAMP_KEY)?.to_timestamp()
    }

    fn timestamp_start(&self) -> Option<Timestamp> {
        self.get(TIMESTAMP_START_KEY)?.to_timestamp()
    }

    fn lvl(&self) -> Option<Level> {
        self.get(LVL_KEY)?.to_level()
    }

    fn module(&self) -> Option<Value> {
        self.get(MODULE_KEY)
    }

    fn trace_id(&self) -> Option<TraceId> {
        self.get(TRACE_ID_KEY)?.to_trace_id()
    }

    fn span_id(&self) -> Option<SpanId> {
        self.get(SPAN_ID_KEY)?.to_span_id()
    }

    fn span_parent(&self) -> Option<SpanId> {
        self.get(SPAN_PARENT_KEY)?.to_span_id()
    }

    fn msg(&self) -> Option<Value> {
        self.get(MSG_KEY)
    }

    fn tpl(&self) -> Option<Value> {
        self.get(TPL_KEY)
    }

    fn err(&self) -> Option<Value> {
        self.get(ERR_KEY)
    }
}

impl<P: Props + ?Sized> WellKnown for P {}
