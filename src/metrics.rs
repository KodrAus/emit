use core::ops::ControlFlow;

use emit_core::{
    extent::{Extent, ToExtent},
    path::Path,
    props::{ByRef, ErasedProps, Props},
    str::{Str, ToStr},
    value::{ToValue, Value},
    well_known::{KEY_METRIC_AGG, KEY_METRIC_NAME, KEY_METRIC_VALUE},
};

pub struct Metric<'a, P> {
    module: Path<'a>,
    extent: Option<Extent>,
    name: Str<'a>,
    agg: Str<'a>,
    value: Value<'a>,
    props: P,
}

impl<'a, P> Metric<'a, P> {
    pub fn new(
        module: impl Into<Path<'a>>,
        extent: impl ToExtent,
        name: impl Into<Str<'a>>,
        agg: impl Into<Str<'a>>,
        value: impl Into<Value<'a>>,
        props: P,
    ) -> Self {
        Metric {
            module: module.into(),
            extent: extent.to_extent(),
            name: name.into(),
            agg: agg.into(),
            value: value.into(),
            props,
        }
    }

    pub fn module(&self) -> &Path<'a> {
        &self.module
    }

    pub fn with_module(mut self, module: impl Into<Path<'a>>) -> Self {
        self.module = module.into();
        self
    }

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
            module: self.module,
            extent: self.extent,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }
}

impl<'a, P: Props> Metric<'a, P> {
    pub fn by_ref<'b>(&'b self) -> Metric<'b, ByRef<'b, P>> {
        Metric {
            module: self.module.by_ref(),
            extent: self.extent.clone(),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Metric<'b, &'b dyn ErasedProps> {
        Metric {
            module: self.module.by_ref(),
            extent: self.extent.clone(),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: &self.props,
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
        for_each(KEY_METRIC_NAME.to_str(), self.name.to_value())?;
        for_each(KEY_METRIC_AGG.to_str(), self.agg.to_value())?;
        for_each(KEY_METRIC_VALUE.to_str(), self.value.by_ref())?;

        self.props.for_each(for_each)
    }
}
