use crate::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::Event, filter::Filter,
    id::IdGen, props::Props, timestamp::Timestamp,
};

#[derive(Debug, Clone, Copy)]
pub struct Ambient<TEmitter = Empty, TFilter = Empty, TCtxt = Empty, TClock = Empty, TIdGen = Empty>
{
    emitter: TEmitter,
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
            emitter: Empty,
            filter: Empty,
            ctxt: Empty,
            clock: Empty,
            id_gen: Empty,
        }
    }
}

impl<TEmitter, TFilter, TCtxt, TClock, TIdGen> Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen> {
    pub fn emitter(&self) -> &TEmitter {
        &self.emitter
    }

    pub fn with_emitter<U>(self, emitter: U) -> Ambient<U, TFilter, TCtxt, TClock, TIdGen> {
        Ambient {
            emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn with_filter<U>(self, filter: U) -> Ambient<TEmitter, U, TCtxt, TClock, TIdGen> {
        Ambient {
            emitter: self.emitter,
            filter,
            ctxt: self.ctxt,
            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn with_ctxt<U>(self, ctxt: U) -> Ambient<TEmitter, TFilter, U, TClock, TIdGen> {
        Ambient {
            emitter: self.emitter,
            filter: self.filter,
            ctxt,

            clock: self.clock,
            id_gen: self.id_gen,
        }
    }

    pub fn clock(&self) -> &TClock {
        &self.clock
    }

    pub fn with_clock<U>(self, clock: U) -> Ambient<TEmitter, TFilter, TCtxt, U, TIdGen> {
        Ambient {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,

            clock,
            id_gen: self.id_gen,
        }
    }

    pub fn id_gen(&self) -> &TIdGen {
        &self.id_gen
    }

    pub fn with_id_gen<U>(self, id_gen: U) -> Ambient<TEmitter, TFilter, TCtxt, TClock, U> {
        Ambient {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,

            clock: self.clock,
            id_gen,
        }
    }
}

impl<TEmitter: Emitter, TFilter, TCtxt, TClock, TIdGen> Emitter
    for Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>
{
    fn emit<P: Props>(&self, evt: &Event<P>) {
        self.emitter.emit(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) {
        self.emitter.blocking_flush(timeout)
    }
}

impl<TEmitter, TFilter: Filter, TCtxt, TClock, TIdGen> Filter
    for Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>
{
    fn matches<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches(evt)
    }
}

impl<TEmitter, TFilter, TCtxt: Ctxt, TClock, TIdGen> Ctxt
    for Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>
{
    type Props = TCtxt::Props;
    type Frame = TCtxt::Frame;

    fn open<P: Props>(&self, props: P) -> Self::Frame {
        self.ctxt.open(props)
    }

    fn enter(&self, scope: &mut Self::Frame) {
        self.ctxt.enter(scope)
    }

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.ctxt.with_current(with)
    }

    fn exit(&self, scope: &mut Self::Frame) {
        self.ctxt.exit(scope)
    }

    fn close(&self, span: Self::Frame) {
        self.ctxt.close(span)
    }
}

impl<TEmitter, TFilter, TCtxt, TClock: Clock, TIdGen> Clock
    for Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>
{
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl<TEmitter, TFilter, TCtxt, TClock, TIdGen: IdGen> IdGen
    for Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>
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
        clock::ErasedClock, ctxt::ErasedCtxt, emitter::ErasedEmitter, filter::ErasedFilter,
        id::ErasedIdGen,
    };

    use super::*;

    trait AmbientTarget: Any + ErasedEmitter + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedEmitter + Send + Sync + 'static);
    }

    impl<T: ErasedEmitter + Send + Sync + 'static> AmbientTarget for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedEmitter + Send + Sync + 'static) {
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

    pub fn init<TEmitter, TFilter, TCtxt, TClock, TIdGen>(
        ambient: Ambient<TEmitter, TFilter, TCtxt, TClock, TIdGen>,
    ) -> Option<
        Ambient<
            &'static TEmitter,
            &'static TFilter,
            &'static TCtxt,
            &'static TClock,
            &'static TIdGen,
        >,
    >
    where
        TEmitter: Emitter + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
        TCtxt::Frame: Send + 'static,
        TClock: Clock + Send + Sync + 'static,
        TIdGen: IdGen + Send + Sync + 'static,
    {
        AMBIENT
            .set(Ambient {
                emitter: Box::new(ambient.emitter),
                filter: Box::new(ambient.filter),
                ctxt: Box::new(ambient.ctxt),
                clock: Box::new(ambient.clock),
                id_gen: Box::new(ambient.id_gen),
            })
            .ok()?;

        let ambient = AMBIENT.get()?;

        Some(Ambient {
            emitter: ambient.emitter.as_any().downcast_ref()?,
            filter: ambient.filter.as_any().downcast_ref()?,
            ctxt: ambient.ctxt.as_any().downcast_ref()?,
            clock: ambient.clock.as_any().downcast_ref()?,
            id_gen: ambient.id_gen.as_any().downcast_ref()?,
        })
    }

    pub type Get = Option<
        Ambient<
            &'static (dyn ErasedEmitter + Send + Sync),
            &'static (dyn ErasedFilter + Send + Sync),
            &'static (dyn ErasedCtxt + Send + Sync),
            &'static (dyn ErasedClock + Send + Sync),
            &'static (dyn ErasedIdGen + Send + Sync),
        >,
    >;

    pub fn get() -> Get {
        let ambient = AMBIENT.get()?;

        Some(Ambient {
            emitter: ambient.emitter.as_super(),
            filter: ambient.filter.as_super(),
            ctxt: ambient.ctxt.as_super(),
            clock: ambient.clock.as_super(),
            id_gen: ambient.id_gen.as_super(),
        })
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
