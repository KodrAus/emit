use crate::{
    id::{SpanId, TraceId},
    level::Level,
    props::Props,
    value::Value,
};

pub const ERR_KEY: &'static str = "#err";

pub const TIMESTAMP_KEY: &'static str = "#ts";
pub const TIMESTAMP_START_KEY: &'static str = "#tss";

pub const LEVEL_KEY: &'static str = "#lvl";
pub const MESSAGE_KEY: &'static str = "#msg";
pub const TEMPLATE_KEY: &'static str = "#tpl";

pub const SPAN_KEY: &'static str = "#sp";
pub const SPAN_PARENT_KEY: &'static str = "#spp";
pub const TRACE_KEY: &'static str = "#tr";

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

pub trait WellKnown {
    fn level(&self) -> Option<Level>;

    fn trace_id(&self) -> Option<TraceId>;

    fn span_id(&self) -> Option<SpanId>;

    fn parent_span_id(&self) -> Option<SpanId>;

    fn err(&self) -> Option<Value>;
}

impl<P: Props> WellKnown for P {
    fn level(&self) -> Option<Level> {
        self.get(LEVEL_KEY)?.to_level()
    }

    fn trace_id(&self) -> Option<TraceId> {
        self.get(TRACE_KEY)?.to_trace_id()
    }

    fn span_id(&self) -> Option<SpanId> {
        self.get(SPAN_KEY)?.to_span_id()
    }

    fn parent_span_id(&self) -> Option<SpanId> {
        self.get(SPAN_PARENT_KEY)?.to_span_id()
    }

    fn err(&self) -> Option<Value> {
        self.get(ERR_KEY)
    }
}
