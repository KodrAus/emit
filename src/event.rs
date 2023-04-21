use std::time::Duration;

use crate::{
    props::{ByRef, Chain, ErasedProps},
    Props, Tpl,
};

#[derive(Clone, Copy)]
pub struct Ts(Duration);

impl Ts {
    pub fn new(time_since_unix_epoch: Duration) -> Self {
        Ts(time_since_unix_epoch)
    }
}

#[derive(Clone, Copy)]
pub enum Lvl {
    Debug,
    Info,
    Warn,
    Error,
}

pub struct Head<'a> {
    pub ts: Option<Ts>,
    pub lvl: Lvl,
    pub tpl: Tpl<'a>,
}

impl<'a> Head<'a> {
    pub fn by_ref<'b>(&'b self) -> Head<'b> {
        Head {
            ts: self.ts,
            lvl: self.lvl,
            tpl: self.tpl.by_ref(),
        }
    }
}

pub struct Event<'a, P = &'a dyn ErasedProps> {
    pub head: Head<'a>,
    pub props: P,
}

impl<'a, P: Props> Event<'a, P> {
    pub fn chain<U: Props>(self, other: U) -> Event<'a, Chain<P, U>> {
        Event {
            head: self.head,
            props: self.props.chain(other),
        }
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            head: self.head.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b> {
        Event {
            head: self.head.by_ref(),
            props: &self.props,
        }
    }
}
