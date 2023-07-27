use core::{fmt, ops::ControlFlow};

use crate::{
    key::{Key, ToKey},
    props::{ByRef, Chain, ErasedProps, Props},
    template::{Render, Template},
    time::Extent,
    value::{ToValue, Value},
    well_known::{MESSAGE_KEY, TEMPLATE_KEY, TIMESTAMP_KEY, TIMESTAMP_START_KEY},
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

        self.all_props().for_each(|k, v| {
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

    pub fn message(&self) -> Render<&P> {
        self.tpl.render(&self.props)
    }

    pub fn template(&self) -> Template {
        self.tpl.by_ref()
    }

    pub fn extent(&self) -> Option<&Extent> {
        self.ts.as_ref()
    }

    pub fn chain<U: Props>(self, other: U) -> Event<'a, Chain<P, U>> {
        Event {
            ts: self.ts,
            tpl: self.tpl,
            props: self.props.chain(other),
        }
    }

    pub fn all_props(&self) -> AllProps<P> {
        AllProps {
            ts: self.ts.clone(),
            tpl: self.tpl.by_ref(),
            msg: self.message(),
            props: self.props.by_ref(),
        }
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

pub struct AllProps<'a, P> {
    ts: Option<Extent>,
    tpl: Template<'a>,
    msg: Render<'a, &'a P>,
    props: ByRef<'a, P>,
}

impl<'a, P: Props> Props for AllProps<'a, P> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        if let Some(ref ts) = self.ts {
            if let Some(start) = ts.start() {
                for_each(TIMESTAMP_START_KEY.to_key(), start.to_value());
            }

            for_each(TIMESTAMP_KEY.to_key(), ts.end().to_value());
        }

        for_each(TEMPLATE_KEY.to_key(), self.tpl.to_value());
        for_each(MESSAGE_KEY.to_key(), self.tpl.to_value());

        self.props.for_each(for_each);
    }
}
