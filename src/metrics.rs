use core::ops::ControlFlow;

use emit_core::{
    empty::Empty,
    extent::{Extent, ToExtent},
    props::Props,
    str::{Str, ToStr},
    value::{ToValue, Value},
    well_known::{METRIC_AGG_KEY, METRIC_NAME_KEY, METRIC_VALUE_KEY},
};

pub struct Metric<'a, P = Empty> {
    extent: Option<Extent>,
    name: Str<'a>,
    agg: Str<'a>,
    value: Value<'a>,
    props: P,
}

impl<'a> Metric<'a> {
    pub fn new(
        name: impl Into<Str<'a>>,
        agg: impl Into<Str<'a>>,
        value: impl Into<Value<'a>>,
    ) -> Self {
        Metric {
            extent: None,
            name: name.into(),
            agg: agg.into(),
            value: value.into(),
            props: Empty,
        }
    }
}

impl<'a, P> Metric<'a, P> {
    pub fn name(&self) -> &Str<'a> {
        &self.name
    }

    pub fn with_name(mut self, name: impl Into<Str<'a>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn agg(&self) -> &Str<'a> {
        &self.agg
    }

    pub fn with_agg(mut self, agg: impl Into<Str<'a>>) -> Self {
        self.agg = agg.into();
        self
    }

    pub fn value(&self) -> &Value<'a> {
        &self.value
    }

    pub fn with_value(mut self, value: impl Into<Value<'a>>) -> Self {
        self.value = value.into();
        self
    }

    pub fn with_extent(mut self, extent: impl ToExtent) -> Self {
        self.extent = extent.to_extent();
        self
    }

    pub fn extent(&self) -> &Option<Extent> {
        &self.extent
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn with_props<U>(self, props: U) -> Metric<'a, U> {
        Metric {
            extent: self.extent,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }
}

impl<'a, P> ToExtent for Metric<'a, P> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent.clone()
    }
}

impl<'a, P: Props> Props for Metric<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(METRIC_NAME_KEY.to_str(), self.name.to_value())?;
        for_each(METRIC_AGG_KEY.to_str(), self.agg.to_value())?;
        for_each(METRIC_VALUE_KEY.to_str(), self.value.by_ref())?;

        self.props.for_each(for_each)
    }
}
