use std::sync::OnceLock;

use crate::{
    ctxt::{Ctxt, ErasedCtxt},
    empty::Empty,
    filter::{ErasedFilter, Filter},
    platform::{DefaultCtxt, Platform},
    target::ErasedTarget,
    Ambient, Target,
};

static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedCtxt + Send + Sync>,
    >,
> = OnceLock::new();

pub(super) fn get() -> Option<&'static Ambient<impl Target, impl Filter, impl Ctxt>> {
    AMBIENT.get()
}

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
            platform: Default::default(),
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
    pub fn init(self) -> Init<&'static TTarget, &'static TFilter, &'static TCtxt> {
        let target = Box::new(self.target);
        let filter = Box::new(self.filter);
        let ctxt = Box::new(self.ctxt);

        AMBIENT
            .set(Ambient {
                target,
                filter,
                ctxt,
                platform: self.platform,
            })
            .map_err(|_| "`emit` is already initialized")
            .unwrap();

        let ambient: &'static _ = AMBIENT.get().unwrap();

        Init {
            // SAFETY: The cell is guaranteed to contain values of the given type
            target: unsafe { &*(&*ambient.target as *const _ as *const TTarget) },
            filter: unsafe { &*(&*ambient.filter as *const _ as *const TFilter) },
            ctxt: unsafe { &*(&*ambient.ctxt as *const _ as *const TCtxt) },
        }
    }
}

pub struct Init<TTarget = DefaultTarget, TFilter = DefaultFilter, TCtxt = DefaultCtxt> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
}

impl<TTarget, TFilter, TCtxt> Init<TTarget, TFilter, TCtxt> {
    pub fn target(&self) -> &TTarget {
        &self.target
    }

    pub fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }
}
