use core::{fmt, ops::ControlFlow, str::FromStr};

use crate::{
    key::{Key, ToKey},
    props::Props,
    value::{ToValue, Value},
    well_known::{METRIC_KIND_KEY, METRIC_NAME_KEY, METRIC_VALUE_KEY},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metric<'m> {
    name: Key<'m>,
    kind: MetricKind,
}

impl<'m> Metric<'m> {
    pub const fn new(name: Key<'m>, kind: MetricKind) -> Self {
        Metric { name, kind }
    }

    pub const fn counter(name: Key<'m>) -> Self {
        Metric::new(name, MetricKind::Counter)
    }

    pub fn name(&self) -> &Key<'m> {
        &self.name
    }

    pub fn kind(&self) -> MetricKind {
        self.kind
    }

    pub fn by_ref(&self) -> Metric {
        Metric {
            name: self.name.by_ref(),
            kind: self.kind,
        }
    }

    pub fn read<'a>(&'a self, v: Value<'a>) -> Reading<'a> {
        Reading::new(self.by_ref(), v)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    impl<'m> Metric<'m> {
        pub fn to_owned(&self) -> Metric<'static> {
            Metric {
                name: self.name.to_owned(),
                kind: self.kind,
            }
        }
    }
}

pub struct Reading<'m> {
    metric: Metric<'m>,
    value: Value<'m>,
}

impl<'m> Reading<'m> {
    pub fn new(metric: Metric<'m>, value: Value<'m>) -> Self {
        Reading { metric, value }
    }
}

impl<'m> Props for Reading<'m> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(METRIC_NAME_KEY.to_key(), self.metric.name.to_value())?;
        for_each(METRIC_KIND_KEY.to_key(), self.metric.kind.to_value())?;
        for_each(METRIC_VALUE_KEY.to_key(), self.value.by_ref())?;

        ControlFlow::Continue(())
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Counter,
}

impl fmt::Debug for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MetricKind::Counter => "counter",
        })
    }
}

impl FromStr for MetricKind {
    type Err = ParseMetricKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("counter") {
            return Ok(MetricKind::Counter);
        }

        Err(ParseMetricKindError {})
    }
}

#[derive(Debug)]
pub struct ParseMetricKindError {}

impl ToValue for MetricKind {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> Value<'v> {
    pub fn to_metric_kind(&self) -> Option<MetricKind> {
        self.downcast_ref::<MetricKind>()
            .copied()
            .or_else(|| self.parse())
    }
}
