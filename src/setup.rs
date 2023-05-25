use emit_core::{ambient::Ambient, ctxt::Ctxt, empty::Empty, filter::Filter, target::Target};

use crate::platform::{DefaultCtxt, Platform};

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
        let ambient = emit_core::ambient::init(
            Ambient::new()
                .with_target(self.target)
                .with_filter(self.filter)
                .with_ctxt(self.ctxt)
                .with_clock(self.platform.clock)
                .with_gen_id(self.platform.gen_id),
        )
        .expect("already initialized");

        Init {
            target: *ambient.target(),
            filter: *ambient.filter(),
            ctxt: *ambient.ctxt(),
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
