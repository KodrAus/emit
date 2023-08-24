use crate::{
    clock::Clock, ctxt::Ctxt, empty::Empty, event::Event, filter::Filter, id::IdGen, props::Props,
    target::Target, timestamp::Timestamp,
};

#[derive(Debug, Clone, Copy)]
pub struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty, TClock = Empty, TIdGen = Empty>
{
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    clock: TClock,
    id_gen: TIdGen,
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
            id_gen: Empty,
        }
    }
}

impl<TTarget, TFilter, TCtxt, TClock, TIdGen> Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen> {
    pub fn target(&self) -> &TTarget {
        &self.target
    }

    pub fn with_target<U>(self, target: U) -> Ambient<U, TFilter, TCtxt, TClock, TIdGen> {
        Ambient {
            target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn with_filter<U>(self, filter: U) -> Ambient<TTarget, U, TCtxt, TClock, TIdGen> {
        Ambient {
            target: self.target,
            filter,
            ctxt: self.ctxt,
            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn with_ctxt<U>(self, ctxt: U) -> Ambient<TTarget, TFilter, U, TClock, TIdGen> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt,
            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn clock(&self) -> &TClock {
        &self.clock
    }

    pub fn with_clock<U>(self, clock: U) -> Ambient<TTarget, TFilter, TCtxt, U, TIdGen> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock,
            id_gen: self.id_gen,
        }
    }

    pub fn id_gen(&self) -> &TIdGen {
        &self.id_gen
    }

    pub fn with_id_gen<U>(self, id_gen: U) -> Ambient<TTarget, TFilter, TCtxt, TClock, U> {
        Ambient {
            target: self.target,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            id_gen,
        }
    }
}

impl<TTarget: Target, TFilter, TCtxt, TClock, TIdGen> Target
    for Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>
{
    fn event<P: Props>(&self, evt: &Event<P>) {
        self.target.event(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) {
        self.target.blocking_flush(timeout)
    }
}

impl<TTarget, TFilter: Filter, TCtxt, TClock, TIdGen> Filter
    for Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>
{
    fn matches<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches(evt)
    }
}

impl<TTarget, TFilter, TCtxt: Ctxt, TClock, TIdGen> Ctxt
    for Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>
{
    type CurrentProps = TCtxt::CurrentProps;
    type LocalFrame = TCtxt::LocalFrame;

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        self.ctxt.with_current(with)
    }

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        self.ctxt.open(props)
    }

    fn enter(&self, scope: &mut Self::LocalFrame) {
        self.ctxt.enter(scope)
    }

    fn exit(&self, scope: &mut Self::LocalFrame) {
        self.ctxt.exit(scope)
    }

    fn close(&self, span: Self::LocalFrame) {
        self.ctxt.close(span)
    }
}

impl<TTarget, TFilter, TCtxt, TClock: Clock, TIdGen> Clock
    for Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>
{
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl<TTarget, TFilter, TCtxt, TClock, TIdGen: IdGen> IdGen
    for Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>
{
    fn new_trace_id(&self) -> Option<crate::id::TraceId> {
        self.id_gen.new_trace_id()
    }

    fn new_span_id(&self) -> Option<crate::id::SpanId> {
        self.id_gen.new_span_id()
    }
}

#[cfg(not(feature = "std"))]
pub type Get = Option<&'static Ambient>;

#[cfg(not(feature = "std"))]
pub fn get() -> Get {
    None::<&'static Ambient>
}

#[cfg(feature = "std")]
mod std_support {
    use core::any::Any;
    use std::sync::OnceLock;

    use crate::{
        clock::ErasedClock, ctxt::ErasedCtxt, filter::ErasedFilter, id::ErasedIdGen,
        target::ErasedTarget,
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

    trait AmbientGenId: Any + ErasedIdGen + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedIdGen + Send + Sync + 'static);
    }

    impl<T: ErasedIdGen + Send + Sync + 'static> AmbientGenId for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedIdGen + Send + Sync + 'static) {
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

    pub fn init<TTarget, TFilter, TCtxt, TClock, TIdGen>(
        ambient: Ambient<TTarget, TFilter, TCtxt, TClock, TIdGen>,
    ) -> Option<
        Ambient<
            &'static TTarget,
            &'static TFilter,
            &'static TCtxt,
            &'static TClock,
            &'static TIdGen,
        >,
    >
    where
        TTarget: Target + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
        TCtxt::LocalFrame: Send + 'static,
        TClock: Clock + Send + Sync + 'static,
        TIdGen: IdGen + Send + Sync + 'static,
    {
        AMBIENT
            .set(Ambient {
                target: Box::new(ambient.target),
                filter: Box::new(ambient.filter),
                ctxt: Box::new(ambient.ctxt),
                clock: Box::new(ambient.clock),
                id_gen: Box::new(ambient.id_gen),
            })
            .ok()?;

        let ambient = AMBIENT.get()?;

        Some(Ambient {
            target: ambient.target.as_any().downcast_ref()?,
            filter: ambient.filter.as_any().downcast_ref()?,
            ctxt: ambient.ctxt.as_any().downcast_ref()?,
            clock: ambient.clock.as_any().downcast_ref()?,
            id_gen: ambient.id_gen.as_any().downcast_ref()?,
        })
    }

    pub type Get = Option<
        Ambient<
            &'static (dyn ErasedTarget + Send + Sync),
            &'static (dyn ErasedFilter + Send + Sync),
            &'static (dyn ErasedCtxt + Send + Sync),
            &'static (dyn ErasedClock + Send + Sync),
            &'static (dyn ErasedIdGen + Send + Sync),
        >,
    >;

    pub fn get() -> Get {
        let ambient = AMBIENT.get()?;

        Some(Ambient {
            target: ambient.target.as_super(),
            filter: ambient.filter.as_super(),
            ctxt: ambient.ctxt.as_super(),
            clock: ambient.clock.as_super(),
            id_gen: ambient.id_gen.as_super(),
        })
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
