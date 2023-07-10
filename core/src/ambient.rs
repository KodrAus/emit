#[derive(Debug, Clone, Copy)]
pub struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty, TClock = Empty, TGenId = Empty>
{
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    clock: TClock,
    gen_id: TGenId,
}

impl Default for Ambient {
    fn default() -> Self {
        Ambient::new()
    }
}

impl Ambient {
    pub fn new() -> Ambient {
        Ambient {
            target: Empty,
            filter: Empty,
            ctxt: Empty,
            clock: Empty,
            gen_id: Empty,
        }
    }
}

impl<TTarget, TFilter, TCtxt, TClock, TGenId> Ambient<TTarget, TFilter, TCtxt, TClock, TGenId> {
    pub fn target(&self) -> &TTarget {
        &self.target
    }

    pub fn with_target<U>(self, target: U) -> Ambient<U, TFilter, TCtxt, TClock, TGenId> {
        Ambient {
            target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            gen_id: self.gen_id,
        }
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn with_filter<U>(self, filter: U) -> Ambient<TTarget, U, TCtxt, TClock, TGenId> {
        Ambient {
            target: self.target,
            filter,
            ctxt: self.ctxt,
            clock: self.clock,
            gen_id: self.gen_id,
        }
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn with_ctxt<U>(self, ctxt: U) -> Ambient<TTarget, TFilter, U, TClock, TGenId> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt,
            clock: self.clock,
            gen_id: self.gen_id,
        }
    }

    pub fn clock(&self) -> &TClock {
        &self.clock
    }

    pub fn with_clock<U>(self, clock: U) -> Ambient<TTarget, TFilter, TCtxt, U, TGenId> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock,
            gen_id: self.gen_id,
        }
    }

    pub fn gen_id(&self) -> &TGenId {
        &self.gen_id
    }

    pub fn with_gen_id<U>(self, gen_id: U) -> Ambient<TTarget, TFilter, TCtxt, TClock, U> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            gen_id,
        }
    }
}

impl<TTarget: Target, TFilter, TCtxt, TClock, TGenId> Target
    for Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>
{
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.target.emit_event(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) {
        self.target.blocking_flush(timeout)
    }
}

impl<TTarget, TFilter: Filter, TCtxt, TClock, TGenId> Filter
    for Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>
{
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches_event(evt)
    }
}

impl<TTarget, TFilter, TCtxt: Ctxt, TClock, TGenId> Ctxt
    for Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>
{
    type Props = TCtxt::Props;
    type Span = TCtxt::Span;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        self.ctxt.with_current(with)
    }

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span {
        self.ctxt.open(ts, id, tpl, props)
    }

    fn enter(&self, scope: &mut Self::Span) {
        self.ctxt.enter(scope)
    }

    fn exit(&self, scope: &mut Self::Span) {
        self.ctxt.exit(scope)
    }

    fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
        self.ctxt.close(ts, span)
    }
}

impl<TTarget, TFilter, TCtxt, TClock: Clock, TGenId> Clock
    for Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>
{
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl<TTarget, TFilter, TCtxt, TClock, TGenId: GenId> GenId
    for Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>
{
    fn gen(&self) -> Id {
        self.gen_id.gen()
    }

    fn gen_trace(&self) -> Option<crate::id::TraceId> {
        self.gen_id.gen_trace()
    }

    fn gen_span(&self) -> Option<crate::id::SpanId> {
        self.gen_id.gen_span()
    }
}

#[cfg(not(feature = "std"))]
pub fn get() -> Option<&'static Ambient<impl Target, impl Filter, impl Ctxt, impl Clock, impl GenId>>
{
    None::<&'static Ambient>
}

#[cfg(feature = "std")]
mod std_support {
    use core::any::Any;
    use std::sync::OnceLock;

    use crate::{
        ctxt::ErasedCtxt, filter::ErasedFilter, id::ErasedGenId, target::ErasedTarget,
        time::ErasedClock,
    };

    use super::*;

    trait AmbientTarget: Any + ErasedTarget + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedTarget + Send + Sync + 'static);
    }

    impl<T: ErasedTarget + Send + Sync + 'static> AmbientTarget for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedTarget + Send + Sync + 'static) {
            self
        }
    }

    trait AmbientFilter: Any + ErasedFilter + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedFilter + Send + Sync + 'static);
    }

    impl<T: ErasedFilter + Send + Sync + 'static> AmbientFilter for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedFilter + Send + Sync + 'static) {
            self
        }
    }

    trait AmbientCtxt: Any + ErasedCtxt + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedCtxt + Send + Sync + 'static);
    }

    impl<T: ErasedCtxt + Send + Sync + 'static> AmbientCtxt for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedCtxt + Send + Sync + 'static) {
            self
        }
    }

    trait AmbientClock: Any + ErasedClock + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedClock + Send + Sync + 'static);
    }

    impl<T: ErasedClock + Send + Sync + 'static> AmbientClock for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedClock + Send + Sync + 'static) {
            self
        }
    }

    trait AmbientGenId: Any + ErasedGenId + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedGenId + Send + Sync + 'static);
    }

    impl<T: ErasedGenId + Send + Sync + 'static> AmbientGenId for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedGenId + Send + Sync + 'static) {
            self
        }
    }

    static AMBIENT: OnceLock<
        Ambient<
            Box<dyn AmbientTarget + Send + Sync>,
            Box<dyn AmbientFilter + Send + Sync>,
            Box<dyn AmbientCtxt + Send + Sync>,
            Box<dyn AmbientClock + Send + Sync>,
            Box<dyn AmbientGenId + Send + Sync>,
        >,
    > = OnceLock::new();

    pub fn init<TTarget, TFilter, TCtxt, TClock, TGenId>(
        ambient: Ambient<TTarget, TFilter, TCtxt, TClock, TGenId>,
    ) -> Option<
        Ambient<
            &'static TTarget,
            &'static TFilter,
            &'static TCtxt,
            &'static TClock,
            &'static TGenId,
        >,
    >
    where
        TTarget: Target + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
        TCtxt::Span: Send + 'static,
        TClock: Clock + Send + Sync + 'static,
        TGenId: GenId + Send + Sync + 'static,
    {
        AMBIENT
            .set(Ambient {
                target: Box::new(ambient.target),
                filter: Box::new(ambient.filter),
                ctxt: Box::new(ambient.ctxt),
                clock: Box::new(ambient.clock),
                gen_id: Box::new(ambient.gen_id),
            })
            .ok()?;

        let ambient = AMBIENT.get()?;

        Some(Ambient {
            target: ambient.target.as_any().downcast_ref()?,
            filter: ambient.filter.as_any().downcast_ref()?,
            ctxt: ambient.ctxt.as_any().downcast_ref()?,
            clock: ambient.clock.as_any().downcast_ref()?,
            gen_id: ambient.gen_id.as_any().downcast_ref()?,
        })
    }

    pub fn get() -> Option<
        Ambient<
            impl Target + Send + Sync + Copy,
            impl Filter + Send + Sync + Copy,
            impl Ctxt + Send + Sync + Copy,
            impl Clock + Send + Sync + Copy,
            impl GenId + Send + Sync + Copy,
        >,
    > {
        let ambient = AMBIENT.get()?;

        Some(Ambient {
            target: ambient.target.as_super(),
            filter: ambient.filter.as_super(),
            ctxt: ambient.ctxt.as_super(),
            clock: ambient.clock.as_super(),
            gen_id: ambient.gen_id.as_super(),
        })
    }
}

use crate::template::Template;
use crate::{
    ctxt::Ctxt,
    empty::Empty,
    event::Event,
    filter::Filter,
    id::{GenId, Id},
    props::Props,
    target::Target,
    time::{Clock, Timestamp},
};

#[cfg(feature = "std")]
pub use self::std_support::*;
