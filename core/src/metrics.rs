use core::ops::ControlFlow;

use crate::{
    key::{Key, ToKey},
    props::Props,
    value::{ToValue, Value},
    well_known::{
        METRIC_KIND_KEY, METRIC_KIND_MAX, METRIC_KIND_MIN, METRIC_KIND_SUM, METRIC_NAME_KEY,
        METRIC_VALUE_KEY,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metric<'m, T> {
    name: Key<'m>,
    kind: Key<'m>,
    value: T,
}

impl<'m, T> Metric<'m, T> {
    pub const fn new(kind: Key<'m>, name: Key<'m>, value: T) -> Self {
        Metric { name, kind, value }
    }

    pub const fn sum(name: Key<'m>, value: T) -> Self {
        Metric::new(Key::new(METRIC_KIND_SUM), name, value)
    }

    pub fn is_sum(&self) -> bool {
        self.kind() == METRIC_KIND_SUM
    }

    pub const fn min(name: Key<'m>, value: T) -> Self {
        Metric::new(Key::new(METRIC_KIND_MIN), name, value)
    }

    pub fn is_min(&self) -> bool {
        self.kind() == METRIC_KIND_MIN
    }

    pub const fn max(name: Key<'m>, value: T) -> Self {
        Metric::new(Key::new(METRIC_KIND_MAX), name, value)
    }

    pub fn is_max(&self) -> bool {
        self.kind() == METRIC_KIND_MAX
    }

    pub const fn name(&self) -> &Key<'m> {
        &self.name
    }

    pub const fn kind(&self) -> &Key<'m> {
        &self.kind
    }

    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'m, V: ToValue> Props for Metric<'m, V> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(METRIC_NAME_KEY.to_key(), self.name.to_value())?;
        for_each(METRIC_VALUE_KEY.to_key(), self.value.to_value())?;
        for_each(METRIC_KIND_KEY.to_key(), self.kind.to_value())?;

        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use crate::well_known::WellKnown;

    use super::*;

    #[test]
    fn metric_well_known() {
        let metric = Metric::sum(Key::new("metric"), Value::from(1usize));

        let well_known = WellKnown::metric(&metric).unwrap();

        assert_eq!("metric", well_known.name());
        assert_eq!("sum", well_known.kind());
        assert_eq!(1.0, well_known.value().to_f64().unwrap());
    }
}
