use core::time::Duration;

use emit_core::{
    ambient::Ambient,
    ctxt::Ctxt,
    empty::Empty,
    filter::Filter,
    target::{self, Target},
};

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

impl<TTarget: Target, TFilter: Filter, TCtxt: Ctxt> Setup<TTarget, TFilter, TCtxt> {
    pub fn to<UTarget: Target>(self, target: UTarget) -> Setup<UTarget, TFilter, TCtxt> {
        Setup {
            target,
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    pub fn and_to<UTarget: Target>(
        self,
        target: UTarget,
    ) -> Setup<target::And<TTarget, UTarget>, TFilter, TCtxt> {
        Setup {
            target: self.target.and(target),
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    pub fn with<UCtxt: Ctxt>(self, ctxt: UCtxt) -> Setup<TTarget, TFilter, UCtxt> {
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
    TCtxt::LocalFrame: Send + 'static,
{
    #[must_use = "call `blocking_flush(std::time::Duration::from_secs(5))` at the end of `main` to ensure events are flushed."]
    pub fn init(self) -> Init<&'static TTarget> {
        let ambient = emit_core::ambient::init(
            Ambient::new()
                .with_target(self.target)
                .with_filter(self.filter)
                .with_ctxt(self.ctxt)
                .with_clock(self.platform.clock)
                .with_gen_id(self.platform.id_gen),
        )
        .expect("already initialized");

        Init {
            target: *ambient.target(),
        }
    }
}

pub struct Init<TTarget: Target = DefaultTarget> {
    target: TTarget,
}

impl<TTarget: Target> Init<TTarget> {
    pub fn blocking_flush(&self, timeout: Duration) {
        self.target.blocking_flush(timeout);
    }
}
