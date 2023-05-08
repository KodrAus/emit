use crate::{
    adapt::Empty, time::Time, Event, Filter, Props, PropsCtxt, ScopeCtxt, Target, Timestamp,
};

#[cfg(feature = "std")]
use crate::{ctxt::ErasedScopeCtxt, filter::ErasedFilter, target::ErasedTarget, time::ErasedTime};

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedScopeCtxt + Send + Sync>,
        Box<dyn ErasedTime + Send + Sync>,
    >,
> = OnceLock::new();

pub(crate) fn get() -> Option<&'static (impl Target + Filter + ScopeCtxt + Time)> {
    #[cfg(feature = "std")]
    {
        AMBIENT.get()
    }
    #[cfg(not(feature = "std"))]
    {
        None::<&'static Ambient>
    }
}

struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty, TTime = Empty> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    time: TTime,
}

impl<TTarget: Target, TFilter, TCtxt, TTime> Target for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.target.emit_event(evt)
    }
}

impl<TTarget, TFilter: Filter, TCtxt, TTime> Filter for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches_event(evt)
    }
}

impl<TTarget, TFilter, TCtxt: PropsCtxt, TTime> PropsCtxt
    for Ambient<TTarget, TFilter, TCtxt, TTime>
{
    type Props = TCtxt::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.ctxt.with_props(with)
    }
}

impl<TTarget, TFilter, TCtxt: ScopeCtxt, TTime> ScopeCtxt
    for Ambient<TTarget, TFilter, TCtxt, TTime>
{
    type Scope = TCtxt::Scope;

    fn prepare<P: Props>(&self, props: P) -> Self::Scope {
        self.ctxt.prepare(props)
    }

    fn enter(&self, scope: &mut Self::Scope) {
        self.ctxt.enter(scope)
    }

    fn exit(&self, scope: &mut Self::Scope) {
        self.ctxt.exit(scope)
    }
}

impl<TTarget, TFilter, TCtxt, TTime: Time> Time for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn timestamp(&self) -> Option<Timestamp> {
        self.time.timestamp()
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use crate::time::SystemClock;

    #[derive(Default)]
    pub struct Setup {
        target: Option<Box<dyn ErasedTarget + Send + Sync>>,
        filter: Option<Box<dyn ErasedFilter + Send + Sync>>,
        ctxt: Option<Box<dyn ErasedScopeCtxt + Send + Sync>>,
        time: Option<Box<dyn ErasedTime + Send + Sync>>,
    }

    impl Setup {
        pub fn new() -> Self {
            Setup {
                target: None,
                filter: None,
                ctxt: None,
                time: None,
            }
        }

        pub fn to(self, target: impl Target + Send + Sync + 'static) -> Self {
            Setup {
                target: Some(Box::new(target)),
                filter: self.filter,
                ctxt: self.ctxt,
                time: self.time,
            }
        }

        pub fn with<C: ScopeCtxt + Send + Sync + 'static>(self, ctxt: C) -> Self
        where
            C::Scope: Send + 'static,
        {
            Setup {
                target: self.target,
                filter: self.filter,
                ctxt: Some(Box::new(ctxt)),
                time: self.time,
            }
        }

        pub fn init(self) {
            let target = self.target.unwrap_or_else(|| Box::new(Empty));
            let filter = self.filter.unwrap_or_else(|| Box::new(Empty));
            let ctxt = self.ctxt.unwrap_or_else(|| Box::new(Empty));
            let time = self.time.unwrap_or_else(|| Box::new(SystemClock));

            let _ = AMBIENT.set(Ambient {
                target,
                filter,
                ctxt,
                time,
            });
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
