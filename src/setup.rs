use core::time::Duration;

use emit_core::{
    ambient::Ambient,
    ctxt::Ctxt,
    emitter::{self, Emitter},
    empty::Empty,
    filter::Filter,
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
    pub fn to<UEmitter: Emitter>(self, emitter: UEmitter) -> Setup<UEmitter, TFilter, TCtxt> {
        Setup {
            emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    pub fn and_to<UEmitter: Emitter>(
        self,
        emitter: UEmitter,
    ) -> Setup<emitter::And<TEmitter, UEmitter>, TFilter, TCtxt> {
        Setup {
            emitter: self.emitter.and(emitter),
            filter: self.filter,
            ctxt: self.ctxt,

            platform: self.platform,
        }
    }

    pub fn with<UCtxt: Ctxt>(self, ctxt: UCtxt) -> Setup<TEmitter, TFilter, UCtxt> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt,

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
    #[must_use = "call `blocking_flush(std::time::Duration::from_secs(5))` at the end of `main` to ensure events are flushed."]
    pub fn init(self) -> Init<&'static TEmitter, &'static TCtxt> {
        let ambient = emit_core::ambient::init(
            Ambient::new()
                .with_emitter(self.emitter)
                .with_filter(self.filter)
                .with_ctxt(self.ctxt)
                .with_clock(self.platform.clock)
                .with_id_gen(self.platform.id_gen),
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
