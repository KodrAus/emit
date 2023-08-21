use core::{
    fmt,
    ops::{ControlFlow, Range},
};

use crate::{
    extent::Extent,
    key::{Key, ToKey},
    props::{ByRef, Chain, ErasedProps, Props},
    template::{Render, Template},
    time::Timestamp,
    value::{ToValue, Value},
    well_known::{MSG_KEY, TPL_KEY, TSS_KEY, TS_KEY},
};

#[derive(Clone)]
pub struct Event<'a, P> {
    extent: Option<Range<Timestamp>>,
    tpl: Template<'a>,
    props: P,
}

impl<'a, P: Props> fmt::Debug for Event<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Event");

        self.for_each(|k, v| {
            f.field(k.as_str(), &v);

            ControlFlow::Continue(())
        });

        f.finish()
    }
}

impl<'a, P> Event<'a, P> {
    pub fn new(extent: impl Extent, tpl: Template<'a>, props: P) -> Self {
        Event {
            extent: extent.extent(),
            tpl,
            props,
        }
    }

    pub fn extent(&self) -> Option<&Range<Timestamp>> {
        self.extent.as_ref()
    }
}

impl<'a, P: Props> Event<'a, P> {
    pub fn msg(&self) -> Render<&P> {
        self.tpl.render(&self.props)
    }

    pub fn tpl(&self) -> Template {
        self.tpl.by_ref()
    }

    pub fn chain<U: Props>(self, other: U) -> Event<'a, Chain<P, U>> {
        Event {
            extent: self.extent,
            tpl: self.tpl,
            props: self.props.chain(other),
        }
    }

    pub fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        let mut reserved = || {
            if let Some(ref ts) = self.extent {
                if ts.start != ts.end {
                    for_each(TSS_KEY.to_key(), ts.start.to_value())?;
                }

                for_each(TS_KEY.to_key(), ts.end.to_value())?;
            }

            for_each(TPL_KEY.to_key(), self.tpl.to_value())?;
            for_each(MSG_KEY.to_key(), Msg::new_ref(self).to_value())?;

            ControlFlow::Continue(())
        };

        if let ControlFlow::Break(()) = reserved() {
            return;
        }

        self.props.for_each(for_each);
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b, &'b dyn ErasedProps> {
        Event {
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }
}

impl<'a, P: Props> Props for Event<'a, P> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        self.for_each(for_each)
    }
}

#[repr(transparent)]
struct Msg<'a, P>(Event<'a, P>);

impl<'a, P> Msg<'a, P> {
    fn new_ref<'b>(evt: &'b Event<'a, P>) -> &'b Msg<'a, P> {
        unsafe { &*(evt as *const Event<'a, P> as *const Msg<'a, P>) }
    }
}

impl<'a, P: Props> ToValue for Msg<'a, P> {
    fn to_value(&self) -> Value {
        if let Some(msg) = self.0.tpl.as_str() {
            Value::from(msg)
        } else {
            Value::from_display(self)
        }
    }
}

impl<'a, P: Props> fmt::Display for Msg<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0.msg(), f)
    }
}
