#[doc(inline)]
pub use emit_core::well_known::*;
use emit_core::{str::Str, props::Props, value::Value};

use crate::{
    id::{SpanId, TraceId},
    Level,
};

pub trait WellKnown: Props {
    fn lvl(&self) -> Option<Level> {
        self.get(LVL_KEY)?.cast()
    }

    fn location(&self) -> Option<Value> {
        self.get(LOCATION_KEY)
    }

    fn trace_id(&self) -> Option<TraceId> {
        self.get(TRACE_ID_KEY)?.cast()
    }

    fn span_id(&self) -> Option<SpanId> {
        self.get(SPAN_ID_KEY)?.cast()
    }

    fn span_parent(&self) -> Option<SpanId> {
        self.get(SPAN_PARENT_KEY)?.cast()
    }

    fn err(&self) -> Option<Value> {
        self.get(ERR_KEY)
    }

    fn metric_name(&self) -> Option<Str> {
        self.get(METRIC_NAME_KEY)?.to_key()
    }

    fn metric_kind(&self) -> Option<Str> {
        self.get(METRIC_KIND_KEY)?.to_key()
    }

    fn metric_value(&self) -> Option<Value> {
        self.get(METRIC_VALUE_KEY)
    }
}

impl<P: Props + ?Sized> WellKnown for P {}
