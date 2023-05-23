use crate::{
    empty::Empty, id::GenId, platform::Platform, time::Clock, Ctxt, Event, Filter, Id, Props,
    Target, Timestamp,
};

#[cfg(feature = "std")]
use crate::{ctxt::ErasedCtxt, filter::ErasedFilter, target::ErasedTarget};

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
pub mod setup;

#[cfg(feature = "std")]
static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedCtxt + Send + Sync>,
    >,
> = OnceLock::new();

pub(crate) fn get() -> Option<&'static Ambient<impl Target, impl Filter, impl Ctxt>> {
    #[cfg(feature = "std")]
    {
        AMBIENT.get()
    }
    #[cfg(not(feature = "std"))]
    {
        None::<&'static Ambient>
    }
}

pub(crate) struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    platform: Platform,
}

impl<TTarget: Target, TFilter, TCtxt> Target for Ambient<TTarget, TFilter, TCtxt> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.target.emit_event(evt)
    }
}

impl<TTarget, TFilter: Filter, TCtxt> Filter for Ambient<TTarget, TFilter, TCtxt> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches_event(evt)
    }
}

impl<TTarget, TFilter, TCtxt: Ctxt> Ctxt for Ambient<TTarget, TFilter, TCtxt> {
    type Props = TCtxt::Props;
    type Span = TCtxt::Span;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        self.ctxt.with_current(with)
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        self.ctxt.open(id, props)
    }

    fn enter(&self, scope: &mut Self::Span) {
        self.ctxt.enter(scope)
    }

    fn exit(&self, scope: &mut Self::Span) {
        self.ctxt.exit(scope)
    }

    fn close(&self, span: Self::Span) {
        self.ctxt.close(span)
    }
}

impl<TTarget, TFilter, TCtxt> Clock for Ambient<TTarget, TFilter, TCtxt> {
    fn now(&self) -> Option<Timestamp> {
        self.platform.now()
    }
}

impl<TTarget, TFilter, TCtxt> GenId for Ambient<TTarget, TFilter, TCtxt> {
    fn gen_trace(&self) -> Option<crate::id::TraceId> {
        self.platform.gen_trace()
    }

    fn gen_span(&self) -> Option<crate::id::SpanId> {
        self.platform.gen_span()
    }
}
