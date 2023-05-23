use crate::{
    empty::Empty, id::IdGenerator, time::Time, Ctxt, Event, Filter, Id, Props, Target, Timestamp,
};

#[cfg(feature = "std")]
use crate::{
    ctxt::ErasedCtxt, filter::ErasedFilter, id::ErasedIdGenerator, target::ErasedTarget,
    time::ErasedTime,
};

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedCtxt + Send + Sync>,
        Box<dyn ErasedTime + Send + Sync>,
        Box<dyn ErasedIdGenerator + Send + Sync>,
    >,
> = OnceLock::new();

pub(crate) fn get(
) -> Option<&'static Ambient<impl Target, impl Filter, impl Ctxt, impl Time, impl IdGenerator>> {
    #[cfg(feature = "std")]
    {
        AMBIENT.get()
    }
    #[cfg(not(feature = "std"))]
    {
        None::<&'static Ambient>
    }
}

pub struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty, TTime = Empty, TId = Empty> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    time: TTime,
    id_generator: TId,
}

impl<TTarget, TFilter, TCtxt, TTime, TId> Ambient<TTarget, TFilter, TCtxt, TTime, TId> {
    pub fn target(&self) -> &TTarget {
        &self.target
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn time(&self) -> &TTime {
        &self.time
    }

    pub fn id_generator(&self) -> &TId {
        &self.id_generator
    }
}

impl<TTarget: Target, TFilter, TCtxt, TTime, TId> Target
    for Ambient<TTarget, TFilter, TCtxt, TTime, TId>
{
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.target.emit_event(evt)
    }
}

impl<TTarget, TFilter: Filter, TCtxt, TTime, TId> Filter
    for Ambient<TTarget, TFilter, TCtxt, TTime, TId>
{
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches_event(evt)
    }
}

impl<TTarget, TFilter, TCtxt: Ctxt, TTime, TId> Ctxt
    for Ambient<TTarget, TFilter, TCtxt, TTime, TId>
{
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

impl<TTarget, TFilter, TCtxt, TTime: Time, TId> Time
    for Ambient<TTarget, TFilter, TCtxt, TTime, TId>
{
    fn timestamp(&self) -> Option<Timestamp> {
        self.time.timestamp()
    }
}

impl<TTarget, TFilter, TCtxt, TTime, TId: IdGenerator> IdGenerator
    for Ambient<TTarget, TFilter, TCtxt, TTime, TId>
{
    fn trace(&self) -> Option<crate::id::TraceId> {
        self.id_generator.trace()
    }

    fn span(&self) -> Option<crate::id::SpanId> {
        self.id_generator.span()
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use crate::{ctxt::thread_local::ThreadLocalCtxt, time::SystemClock};

    type DefaultTarget = Empty;
    type DefaultFilter = Empty;
    type DefaultCtxt = ThreadLocalCtxt;
    type DefaultTime = SystemClock;

    #[cfg(not(feature = "id-generator"))]
    type DefaultIdGenerator = Empty;
    #[cfg(feature = "id-generator")]
    type DefaultIdGenerator = crate::id::RngIdGenerator;

    pub struct Setup<
        TTarget = DefaultTarget,
        TFilter = DefaultFilter,
        TCtxt = DefaultCtxt,
        TTime = DefaultTime,
        TId = DefaultIdGenerator,
    > {
        target: TTarget,
        filter: TFilter,
        ctxt: TCtxt,
        time: TTime,
        id_generator: TId,
    }

    impl Default for Setup {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Setup {
        pub fn new() -> Self {
            Setup {
                target: Default::default(),
                filter: Default::default(),
                ctxt: Default::default(),
                time: Default::default(),
                id_generator: Default::default(),
            }
        }
    }

    impl<TTarget, TFilter, TCtxt, TTime, TId> Setup<TTarget, TFilter, TCtxt, TTime, TId> {
        pub fn to<UTarget>(self, target: UTarget) -> Setup<UTarget, TFilter, TCtxt, TTime, TId> {
            Setup {
                target,
                filter: self.filter,
                ctxt: self.ctxt,
                time: self.time,
                id_generator: self.id_generator,
            }
        }

        pub fn with<UCtxt>(self, ctxt: UCtxt) -> Setup<TTarget, TFilter, UCtxt, TTime, TId> {
            Setup {
                target: self.target,
                filter: self.filter,
                ctxt,
                time: self.time,
                id_generator: self.id_generator,
            }
        }
    }

    impl<
            TTarget: Target + Send + Sync + 'static,
            TFilter: Filter + Send + Sync + 'static,
            TCtxt: Ctxt + Send + Sync + 'static,
            TTime: Time + Send + Sync + 'static,
            TId: IdGenerator + Send + Sync + 'static,
        > Setup<TTarget, TFilter, TCtxt, TTime, TId>
    where
        TCtxt::Span: Send + 'static,
    {
        pub fn init(
            self,
        ) -> Ambient<&'static TTarget, &'static TFilter, &'static TCtxt, &'static TTime, &'static TId>
        {
            let target = Box::new(self.target);
            let filter = Box::new(self.filter);
            let ctxt = Box::new(self.ctxt);
            let time = Box::new(self.time);
            let id_generator = Box::new(self.id_generator);

            AMBIENT
                .set(Ambient {
                    target,
                    filter,
                    ctxt,
                    time,
                    id_generator,
                })
                .map_err(|_| "`emit` is already initialized")
                .unwrap();

            let ambient: &'static _ = AMBIENT.get().unwrap();

            Ambient {
                // SAFETY: The cell is guaranteed to contain values of the given type
                target: unsafe { &*(&*ambient.target as *const _ as *const TTarget) },
                filter: unsafe { &*(&*ambient.filter as *const _ as *const TFilter) },
                ctxt: unsafe { &*(&*ambient.ctxt as *const _ as *const TCtxt) },
                time: unsafe { &*(&*ambient.time as *const _ as *const TTime) },
                id_generator: unsafe { &*(&*ambient.target as *const _ as *const TId) },
            }
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
