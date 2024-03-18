use core::time::Duration;

use emit_core::{
    ctxt::Ctxt,
    emitter::{self, Emitter},
    empty::Empty,
    filter::Filter,
    runtime::{InternalCtxt, InternalEmitter, InternalFilter},
};

use crate::platform::{DefaultCtxt, Platform};

pub fn setup() -> Setup {
    Setup::default()
}

type DefaultEmitter = Empty;
type DefaultFilter = Empty;

pub struct Setup<TEmitter = DefaultEmitter, TFilter = DefaultFilter, TCtxt = DefaultCtxt> {
    emitter: TEmitter,
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
            emitter: Default::default(),
            filter: Default::default(),
            ctxt: Default::default(),
            platform: Default::default(),
        }
    }
}

impl<TEmitter: Emitter, TFilter: Filter, TCtxt: Ctxt> Setup<TEmitter, TFilter, TCtxt> {
    #[must_use = "call `.init()` to finish setup"]
    pub fn emit_to<UEmitter: Emitter>(self, emitter: UEmitter) -> Setup<UEmitter, TFilter, TCtxt> {
        Setup {
            emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    #[must_use = "call `.init()` to finish setup"]
    pub fn and_emit_to<UEmitter: Emitter>(
        self,
        emitter: UEmitter,
    ) -> Setup<emitter::And<TEmitter, UEmitter>, TFilter, TCtxt> {
        Setup {
            emitter: self.emitter.and_to(emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    #[must_use = "call `.init()` to finish setup"]
    pub fn map_emitter<UEmitter: Emitter>(
        self,
        map: impl FnOnce(TEmitter) -> UEmitter,
    ) -> Setup<UEmitter, TFilter, TCtxt> {
        Setup {
            emitter: map(self.emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    #[must_use = "call `.init()` to finish setup"]
    pub fn emit_when<UFilter: Filter>(self, filter: UFilter) -> Setup<TEmitter, UFilter, TCtxt> {
        Setup {
            emitter: self.emitter,
            filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    #[must_use = "call `.init()` to finish setup"]
    pub fn with_ctxt<UCtxt: Ctxt>(self, ctxt: UCtxt) -> Setup<TEmitter, TFilter, UCtxt> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt,
            platform: self.platform,
        }
    }

    #[must_use = "call `.init()` to finish setup"]
    pub fn map_ctxt<UCtxt: Ctxt>(
        self,
        map: impl FnOnce(TCtxt) -> UCtxt,
    ) -> Setup<TEmitter, TFilter, UCtxt> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: map(self.ctxt),
            platform: self.platform,
        }
    }
}

impl<
        TEmitter: Emitter + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt>
where
    TCtxt::Frame: Send + 'static,
{
    #[must_use = "call `blocking_flush` at the end of `main` to ensure events are flushed."]
    pub fn init(self) -> Init<&'static TEmitter, &'static TCtxt> {
        self.init_slot(emit_core::runtime::shared_slot())
    }

    #[must_use = "call `blocking_flush` at the end of `main` to ensure events are flushed."]
    pub fn init_slot(
        self,
        slot: &'static emit_core::runtime::AmbientSlot,
    ) -> Init<&'static TEmitter, &'static TCtxt> {
        let ambient = slot
            .init(
                emit_core::runtime::Runtime::new()
                    .with_emitter(self.emitter)
                    .with_filter(self.filter)
                    .with_ctxt(self.ctxt)
                    .with_clock(self.platform.clock)
                    .with_rng(self.platform.rng),
            )
            .expect("already initialized");

        Init {
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        }
    }
}

impl<
        TEmitter: InternalEmitter + Send + Sync + 'static,
        TFilter: InternalFilter + Send + Sync + 'static,
        TCtxt: InternalCtxt + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt>
where
    TCtxt::Frame: Send + 'static,
{
    #[must_use = "call `blocking_flush` at the end of `main` (after flushing the main runtime) to ensure events are flushed."]
    pub fn init_internal(self) -> Init<&'static TEmitter, &'static TCtxt> {
        let ambient = emit_core::runtime::internal_slot()
            .init(
                emit_core::runtime::Runtime::new()
                    .with_emitter(self.emitter)
                    .with_filter(self.filter)
                    .with_ctxt(self.ctxt)
                    .with_clock(self.platform.clock)
                    .with_rng(self.platform.rng),
            )
            .expect("already initialized");

        Init {
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        }
    }
}

pub struct Init<TEmitter: Emitter = DefaultEmitter, TCtxt: Ctxt = DefaultCtxt> {
    emitter: TEmitter,
    ctxt: TCtxt,
}

impl<TEmitter: Emitter, TCtxt: Ctxt> Init<TEmitter, TCtxt> {
    pub fn emitter(&self) -> &TEmitter {
        &self.emitter
    }

    pub fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn blocking_flush(&self, timeout: Duration) {
        self.emitter.blocking_flush(timeout);
    }
}
