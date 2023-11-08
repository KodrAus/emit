use core::{fmt, ops::ControlFlow, str::FromStr};

use crate::{
    key::{Key, ToKey},
    props::Props,
    value::{ToValue, Value},
    well_known::{METRIC_KIND_KEY, METRIC_NAME_KEY, METRIC_VALUE_KEY},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metric<'m, T> {
    name: Key<'m>,
    kind: MetricKind,
    value: T,
}

impl<'m, T> Metric<'m, T> {
    pub const fn new(name: Key<'m>, kind: MetricKind, value: T) -> Self {
        Metric { name, kind, value }
    }

    pub const fn name(&self) -> &Key<'m> {
        &self.name
    }

    pub const fn kind(&self) -> MetricKind {
        self.kind
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn sample<'a, U: ToValue>(
        &'a self,
        sample: impl FnOnce(MetricKind, &'a T) -> (MetricKind, U),
    ) -> Metric<'a, U> {
        let (kind, value) = sample(self.kind, &self.value);

        Metric {
            name: self.name.by_ref(),
            kind,
            value,
        }
    }

    pub fn sample_mut<'a, U: ToValue>(
        &'a mut self,
        sample: impl FnOnce(MetricKind, &'a mut T) -> (MetricKind, U),
    ) -> Metric<'a, U> {
        let (kind, value) = sample(self.kind, &mut self.value);

        Metric {
            name: self.name.by_ref(),
            kind,
            value,
        }
    }
}

impl<'m, V: ToValue> Props for Metric<'m, V> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(METRIC_NAME_KEY.to_key(), self.name.to_value())?;
        for_each(METRIC_KIND_KEY.to_key(), self.kind.to_value())?;
        for_each(METRIC_VALUE_KEY.to_key(), self.value.to_value())?;

        ControlFlow::Continue(())
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    Point,
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
            MetricKind::Point => "point",
            MetricKind::Counter => "counter",
        })
    }
}

impl FromStr for MetricKind {
    type Err = ParseMetricKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("point") {
            return Ok(MetricKind::Point);
        }

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

#[cfg(test)]
mod tests {
    use crate::well_known::WellKnown;

    use super::*;

    #[test]
    fn metric_well_known() {
        let metric = Metric::new(Key::new("metric"), MetricKind::Point, Value::from(1usize));

        let well_known = WellKnown::metric(&metric).unwrap();

        assert_eq!("metric", well_known.name());
        assert_eq!(MetricKind::Point, well_known.kind());
        assert_eq!(1, well_known.value().to_usize().unwrap());
    }
}
