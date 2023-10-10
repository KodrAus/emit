use crate::{
    id::{SpanId, TraceId},
    level::Level,
    metrics::MetricKind,
    props::Props,
    value::Value,
};

pub const ERR_KEY: &'static str = "err";
pub const LVL_KEY: &'static str = "lvl";
pub const LOCATION_KEY: &'static str = "loc";
pub const TRACE_ID_KEY: &'static str = "trace_id";
pub const SPAN_ID_KEY: &'static str = "span_id";
pub const SPAN_PARENT_KEY: &'static str = "span_parent";
pub const METRIC_NAME_KEY: &'static str = "metric_name";
pub const METRIC_KIND_KEY: &'static str = "metric_kind";
pub const METRIC_VALUE_KEY: &'static str = "metric_value";

pub trait WellKnown: Props {
    fn lvl(&self) -> Option<Level> {
        self.get(LVL_KEY)?.to_level()
    }

    fn location(&self) -> Option<Value> {
        self.get(LOCATION_KEY)
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

    fn err(&self) -> Option<Value> {
        self.get(ERR_KEY)
    }

    fn metric_name(&self) -> Option<Value> {
        self.get(METRIC_NAME_KEY)
    }

    fn metric_kind(&self) -> Option<MetricKind> {
        self.get(METRIC_KIND_KEY)?.to_metric_kind()
    }

    fn metric_value(&self) -> Option<Value> {
        self.get(METRIC_VALUE_KEY)
    }
}

impl<P: Props + ?Sized> WellKnown for P {}
