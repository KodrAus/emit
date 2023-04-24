use core::{borrow::Borrow, ops::ControlFlow};

use crate::{
    props::{ByRef, Chain, ErasedProps},
    Key, Props, Template, Timestamp, Value,
};

#[derive(Clone, Copy)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}

#[derive(Clone)]
pub struct Event<'a, P = &'a dyn ErasedProps> {
    ts: Option<Timestamp>,
    lvl: Level,
    tpl: Template<'a>,
    props: P,
}

impl<'a, P: Props> Event<'a, P> {
    pub fn new(lvl: Level, ts: Option<Timestamp>, tpl: Template<'a>, props: P) -> Self {
        Event {
            ts,
            lvl,
            tpl,
            props,
        }
    }

    pub fn for_each<'b>(&'b self, for_each: impl FnMut(Key<'b>, Value<'b>) -> ControlFlow<()>) {
        self.props.for_each(for_each)
    }

    pub fn get<'b>(&'b self, k: impl Borrow<str>) -> Option<Value<'b>> {
        self.props.get(k)
    }

    pub fn chain<U: Props>(self, other: U) -> Event<'a, Chain<P, U>> {
        Event {
            ts: self.ts,
            lvl: self.lvl,
            tpl: self.tpl,
            props: self.props.chain(other),
        }
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            ts: self.ts,
            lvl: self.lvl,
            tpl: self.tpl.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b> {
        Event {
            ts: self.ts,
            lvl: self.lvl,
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }
}
