use core::{borrow::Borrow, fmt, ops::ControlFlow};

use crate::{
    empty::Empty,
    props::{ByRef, Chain, ErasedProps},
    template::Render,
    well_known, Id, Key, Props, Template, Timestamp, Value,
};

#[derive(Clone, Copy)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => "INFO",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Debug => "DEBUG",
        })
    }
}

impl Default for Level {
    fn default() -> Self {
        Level::Info
    }
}

#[derive(Clone)]
pub struct Event<'a, P = &'a dyn ErasedProps> {
    ts: Option<Timestamp>,
    id: Id,
    lvl: Level,
    tpl: Template<'a>,
    props: P,
}

impl<'a, P: Props> fmt::Debug for Event<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Event");

        if let Some(ref ts) = self.ts {
            f.field("ts", ts);
        }

        f.field("lvl", &self.lvl).field("msg", &self.msg());

        self.props.for_each(|k, v| {
            f.field(k.as_str(), &v);

            ControlFlow::Continue(())
        });

        f.finish()
    }
}

impl<'a, P: Props> Event<'a, P> {
    pub fn new(ts: Option<Timestamp>, id: Id, lvl: Level, tpl: Template<'a>, props: P) -> Self {
        Event {
            ts,
            id,
            lvl,
            tpl,
            props,
        }
    }

    pub fn ts(&self) -> Option<Timestamp> {
        self.ts
    }

    pub fn lvl(&self) -> Level {
        self.lvl
    }

    pub fn msg<'b>(&'b self) -> Render<'b, &'b P> {
        self.tpl.render().with_props(&self.props)
    }

    pub fn tpl<'b>(&'b self) -> Render<'b, Empty> {
        self.tpl.render()
    }

    pub fn err<'b>(&'b self) -> Option<Value<'b>> {
        self.props.get(well_known::ERR_KEY)
    }

    pub fn id(&self) -> Id {
        self.id
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
            id: self.id,
            lvl: self.lvl,
            tpl: self.tpl,
            props: self.props.chain(other),
        }
    }

    pub fn props(&self) -> &P {
        &self.props
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            ts: self.ts,
            id: self.id,
            lvl: self.lvl,
            tpl: self.tpl.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b> {
        Event {
            ts: self.ts,
            id: self.id,
            lvl: self.lvl,
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }
}
