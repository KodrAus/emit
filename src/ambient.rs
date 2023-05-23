use crate::{
    empty::Empty, id::IdGenerator, platform::Platform, time::Time, Ctxt, Event, Filter, Id, Props,
    Target, Timestamp,
};

#[cfg(feature = "std")]
use crate::{ctxt::ErasedCtxt, filter::ErasedFilter, target::ErasedTarget};

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedCtxt + Send + Sync>,
    >,
> = OnceLock::new();

#[cfg(feature = "std")]
static PLATFORM: OnceLock<Platform> = OnceLock::new();

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

pub struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    platform: &'static Platform,
}

impl<TTarget, TFilter, TCtxt> Ambient<TTarget, TFilter, TCtxt> {
    pub fn target(&self) -> &TTarget {
        &self.target
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn time<'a>(&'a self) -> impl Time + 'a {
        &self.platform
    }

    pub fn id_generator<'a>(&'a self) -> impl IdGenerator + 'a {
        &self.platform
    }
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

impl<TTarget, TFilter, TCtxt> Time for Ambient<TTarget, TFilter, TCtxt> {
    fn timestamp(&self) -> Option<Timestamp> {
        self.platform.timestamp()
    }
}

impl<TTarget, TFilter, TCtxt> IdGenerator for Ambient<TTarget, TFilter, TCtxt> {
    fn trace(&self) -> Option<crate::id::TraceId> {
        self.platform.trace()
    }

    fn span(&self) -> Option<crate::id::SpanId> {
        self.platform.span()
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use crate::platform::DefaultCtxt;

    type DefaultTarget = Empty;
    type DefaultFilter = Empty;

    pub struct Setup<TTarget = DefaultTarget, TFilter = DefaultFilter, TCtxt = DefaultCtxt> {
        target: TTarget,
        filter: TFilter,
        ctxt: TCtxt,
        platform: Platform,
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
                platform: Platform::new(),
            }
        }
    }

    impl<TTarget, TFilter, TCtxt> Setup<TTarget, TFilter, TCtxt> {
        pub fn to<UTarget>(self, target: UTarget) -> Setup<UTarget, TFilter, TCtxt> {
            Setup {
                target,
                filter: self.filter,
                ctxt: self.ctxt,
                platform: self.platform,
            }
        }

        pub fn with<UCtxt>(self, ctxt: UCtxt) -> Setup<TTarget, TFilter, UCtxt> {
            Setup {
                target: self.target,
                filter: self.filter,
                ctxt,
                platform: self.platform,
            }
        }
    }

    impl<
            TTarget: Target + Send + Sync + 'static,
            TFilter: Filter + Send + Sync + 'static,
            TCtxt: Ctxt + Send + Sync + 'static,
        > Setup<TTarget, TFilter, TCtxt>
    where
        TCtxt::Span: Send + 'static,
    {
        pub fn init(self) -> Ambient<&'static TTarget, &'static TFilter, &'static TCtxt> {
            let target = Box::new(self.target);
            let filter = Box::new(self.filter);
            let ctxt = Box::new(self.ctxt);

            PLATFORM
                .set(self.platform)
                .map_err(|_| "`emit` is already initialized")
                .unwrap();

            let platform: &'static _ = PLATFORM.get().unwrap();

            AMBIENT
                .set(Ambient {
                    target,
                    filter,
                    ctxt,
                    platform,
                })
                .map_err(|_| "`emit` is already initialized")
                .unwrap();

            let ambient: &'static _ = AMBIENT.get().unwrap();

            Ambient {
                // SAFETY: The cell is guaranteed to contain values of the given type
                target: unsafe { &*(&*ambient.target as *const _ as *const TTarget) },
                filter: unsafe { &*(&*ambient.filter as *const _ as *const TFilter) },
                ctxt: unsafe { &*(&*ambient.ctxt as *const _ as *const TCtxt) },
                platform,
            }
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
