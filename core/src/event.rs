use core::{fmt, ops::ControlFlow};

use crate::{
    key::{Key, ToKey},
    props::{ByRef, Chain, ErasedProps, Props},
    template::{Render, Template},
    time::{Extent, Timestamp},
    value::{ToValue, Value},
    well_known::{MSG_KEY, TPL_KEY, TSS_KEY, TS_KEY},
};

#[derive(Clone)]
pub struct Event<'a, P> {
    ts: Option<Extent>,
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

impl<'a, P: Props> Event<'a, P> {
    pub fn new(ts: Option<impl Into<Extent>>, tpl: Template<'a>, props: P) -> Self {
        Event {
            ts: ts.map(Into::into),
            tpl,
            props,
        }
    }

    pub fn ts(&self) -> Option<Timestamp> {
        self.extent().map(|ts| *ts.end())
    }

    pub fn tss(&self) -> Option<Timestamp> {
        self.extent().and_then(|ts| ts.start().cloned())
    }

    pub fn extent(&self) -> Option<&Extent> {
        self.ts.as_ref()
    }

    pub fn msg(&self) -> Render<&P> {
        self.tpl.render(&self.props)
    }

    pub fn tpl(&self) -> Template {
        self.tpl.by_ref()
    }

    pub fn chain<U: Props>(self, other: U) -> Event<'a, Chain<P, U>> {
        Event {
            ts: self.ts,
            tpl: self.tpl,
            props: self.props.chain(other),
        }
    }

    pub fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        if let Some(ref ts) = self.ts {
            if let Some(start) = ts.start() {
                for_each(TSS_KEY.to_key(), start.to_value());
            }

            for_each(TS_KEY.to_key(), ts.end().to_value());
        }

        for_each(TPL_KEY.to_key(), self.tpl.to_value());
        for_each(MSG_KEY.to_key(), Msg::new_ref(self).to_value());

        self.props.for_each(for_each);
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            ts: self.ts.clone(),
            tpl: self.tpl.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b, &'b dyn ErasedProps> {
        Event {
            ts: self.ts.clone(),
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
