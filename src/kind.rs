use core::{fmt, str::FromStr};

use emit_core::{
    event::ToEvent,
    filter::Filter,
    props::Props,
    value::{FromValue, ToValue, Value},
    well_known::{EVENT_KIND_METRIC, EVENT_KIND_SPAN, KEY_EVENT_KIND},
};

#[non_exhaustive]
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Kind {
    Span,
    Metric,
}

impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Span => f.write_str(EVENT_KIND_SPAN),
            Kind::Metric => f.write_str(EVENT_KIND_METRIC),
        }
    }
}

impl ToValue for Kind {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for Kind {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<Kind>()
            .copied()
            .or_else(|| value.parse())
    }
}

impl FromStr for Kind {
    type Err = ParseKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case(EVENT_KIND_SPAN) {
            return Ok(Kind::Span);
        }

        if s.eq_ignore_ascii_case(EVENT_KIND_METRIC) {
            return Ok(Kind::Metric);
        }

        Err(ParseKindError {})
    }
}

pub struct ParseKindError {}

pub struct IsKind(Kind);

impl IsKind {
    pub fn new(kind: Kind) -> Self {
        IsKind(kind)
    }
}

pub fn is_span_filter() -> IsKind {
    IsKind::new(Kind::Span)
}

pub fn is_metric_filter() -> IsKind {
    IsKind::new(Kind::Metric)
}

impl Filter for IsKind {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        evt.to_event().props().pull::<Kind, _>(KEY_EVENT_KIND) == Some(self.0)
    }
}
