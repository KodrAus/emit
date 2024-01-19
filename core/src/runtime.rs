use crate::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::Event, filter::Filter,
    props::Props, rng::Rng, timestamp::Timestamp,
};

static SHARED: AmbientSlot = AmbientSlot::new();
static INTERNAL: AmbientInternalSlot = AmbientInternalSlot::new();

pub fn shared() -> &'static AmbientRuntime<'static> {
    SHARED.get()
}

pub fn shared_slot() -> &'static AmbientSlot {
    &SHARED
}

pub fn internal() -> &'static AmbientRuntime<'static> {
    INTERNAL.get()
}

pub fn internal_slot() -> &'static AmbientInternalSlot {
    &INTERNAL
}

#[derive(Debug, Clone, Copy)]
pub struct Runtime<TEmitter = Empty, TFilter = Empty, TCtxt = Empty, TClock = Empty, TRng = Empty> {
    pub(crate) emitter: TEmitter,
    pub(crate) filter: TFilter,
    pub(crate) ctxt: TCtxt,
    pub(crate) clock: TClock,
    pub(crate) rng: TRng,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime::new()
    }
}

impl Runtime {
    pub const fn new() -> Runtime {
        Runtime {
            emitter: Empty,
            filter: Empty,
            ctxt: Empty,
            clock: Empty,
            rng: Empty,
        }
    }
}

impl<TEmitter, TFilter, TCtxt, TClock, TRng> Runtime<TEmitter, TFilter, TCtxt, TClock, TRng> {
    pub const fn emitter(&self) -> &TEmitter {
        &self.emitter
    }

    pub fn with_emitter<U>(self, emitter: U) -> Runtime<U, TFilter, TCtxt, TClock, TRng> {
        self.map_emitter(|_| emitter)
    }

    pub fn map_emitter<U>(
        self,
        emitter: impl FnOnce(TEmitter) -> U,
    ) -> Runtime<U, TFilter, TCtxt, TClock, TRng> {
        Runtime {
            emitter: emitter(self.emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    pub const fn filter(&self) -> &TFilter {
        &self.filter
    }

    pub fn with_filter<U>(self, filter: U) -> Runtime<TEmitter, U, TCtxt, TClock, TRng> {
        self.map_filter(|_| filter)
    }

    pub fn map_filter<U>(
        self,
        filter: impl FnOnce(TFilter) -> U,
    ) -> Runtime<TEmitter, U, TCtxt, TClock, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: filter(self.filter),
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    pub const fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    pub fn with_ctxt<U>(self, ctxt: U) -> Runtime<TEmitter, TFilter, U, TClock, TRng> {
        self.map_ctxt(|_| ctxt)
    }

    pub fn map_ctxt<U>(
        self,
        ctxt: impl FnOnce(TCtxt) -> U,
    ) -> Runtime<TEmitter, TFilter, U, TClock, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: ctxt(self.ctxt),
            clock: self.clock,
            rng: self.rng,
        }
    }

    pub const fn clock(&self) -> &TClock {
        &self.clock
    }

    pub fn with_clock<U>(self, clock: U) -> Runtime<TEmitter, TFilter, TCtxt, U, TRng> {
        self.map_clock(|_| clock)
    }

    pub fn map_clock<U>(
        self,
        clock: impl FnOnce(TClock) -> U,
    ) -> Runtime<TEmitter, TFilter, TCtxt, U, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: clock(self.clock),
            rng: self.rng,
        }
    }

    pub const fn rng(&self) -> &TRng {
        &self.rng
    }

    pub fn with_rng<U>(self, id_gen: U) -> Runtime<TEmitter, TFilter, TCtxt, TClock, U> {
        self.map_rng(|_| id_gen)
    }

    pub fn map_rng<U>(
        self,
        id_gen: impl FnOnce(TRng) -> U,
    ) -> Runtime<TEmitter, TFilter, TCtxt, TClock, U> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: id_gen(self.rng),
        }
    }
}

impl<TEmitter: Emitter, TFilter, TCtxt, TClock, TRng> Emitter
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    fn emit<P: Props>(&self, evt: &Event<P>) {
        self.emitter.emit(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) {
        self.emitter.blocking_flush(timeout)
    }
}

impl<TEmitter, TFilter: Filter, TCtxt, TClock, TRng> Filter
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    fn matches<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches(evt)
    }
}

impl<TEmitter, TFilter, TCtxt: Ctxt, TClock, TRng> Ctxt
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
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

impl<TEmitter, TFilter, TCtxt, TClock: Clock, TRng> Clock
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    fn now(&self) -> Option<Timestamp> {
        self.clock.now()
    }
}

impl<TEmitter, TFilter, TCtxt, TClock, TRng: Rng> Rng
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    fn gen_u64(&self) -> Option<u64> {
        self.rng.gen_u64()
    }
}

pub trait InternalEmitter: Emitter {}

impl InternalEmitter for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalEmitter> InternalEmitter for alloc::boxed::Box<T> {}

pub trait InternalFilter: Filter {}

impl InternalFilter for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalFilter> InternalFilter for alloc::boxed::Box<T> {}

pub trait InternalCtxt: Ctxt {}

impl InternalCtxt for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalCtxt> InternalCtxt for alloc::boxed::Box<T> {}

pub trait InternalClock: Clock {}

impl InternalClock for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalClock> InternalClock for alloc::boxed::Box<T> {}

pub trait InternalRng: Rng {}

impl InternalRng for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalRng> InternalRng for alloc::boxed::Box<T> {}

#[cfg(feature = "std")]
mod std_support {
    use core::any::Any;
    use std::sync::OnceLock;

    use crate::{
        clock::ErasedClock, ctxt::ErasedCtxt, emitter::ErasedEmitter, filter::ErasedFilter,
        rng::ErasedRng,
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

    trait AmbientGenId: Any + ErasedRng + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedRng + Send + Sync + 'static);
    }

    impl<T: ErasedRng + Send + Sync + 'static> AmbientGenId for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedRng + Send + Sync + 'static) {
            self
        }
    }

    pub struct AmbientSlot(OnceLock<AmbientSync>);

    pub struct AmbientInternalSlot(AmbientSlot);

    struct AmbientSync {
        value: AmbientSyncValue,
        runtime: AmbientSyncRuntime,
    }

    type AmbientSyncValue = Runtime<
        Box<dyn AmbientTarget + Send + Sync>,
        Box<dyn AmbientFilter + Send + Sync>,
        Box<dyn AmbientCtxt + Send + Sync>,
        Box<dyn AmbientClock + Send + Sync>,
        Box<dyn AmbientGenId + Send + Sync>,
    >;

    type AmbientSyncRuntime = Runtime<
        *const (dyn ErasedEmitter + Send + Sync),
        *const (dyn ErasedFilter + Send + Sync),
        *const (dyn ErasedCtxt + Send + Sync),
        *const (dyn ErasedClock + Send + Sync),
        *const (dyn ErasedRng + Send + Sync),
    >;

    pub type AmbientRuntime<'a> = Runtime<
        &'a (dyn ErasedEmitter + Send + Sync),
        &'a (dyn ErasedFilter + Send + Sync),
        &'a (dyn ErasedCtxt + Send + Sync),
        &'a (dyn ErasedClock + Send + Sync),
        &'a (dyn ErasedRng + Send + Sync),
    >;

    unsafe impl Send for AmbientSync where AmbientSyncValue: Send {}
    unsafe impl Sync for AmbientSync where AmbientSyncValue: Sync {}

    impl AmbientSlot {
        pub const fn new() -> Self {
            AmbientSlot(OnceLock::new())
        }

        pub fn init<TEmitter, TFilter, TCtxt, TClock, TRng>(
            &self,
            pipeline: Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>,
        ) -> Option<Runtime<&TEmitter, &TFilter, &TCtxt, &TClock, &TRng>>
        where
            TEmitter: Emitter + Send + Sync + 'static,
            TFilter: Filter + Send + Sync + 'static,
            TCtxt: Ctxt + Send + Sync + 'static,
            TCtxt::Frame: Send + 'static,
            TClock: Clock + Send + Sync + 'static,
            TRng: Rng + Send + Sync + 'static,
        {
            self.0
                .set({
                    let value = pipeline
                        .map_emitter(|emitter| {
                            Box::new(emitter) as Box<dyn AmbientTarget + Send + Sync>
                        })
                        .map_filter(|filter| {
                            Box::new(filter) as Box<dyn AmbientFilter + Send + Sync>
                        })
                        .map_ctxt(|ctxt| Box::new(ctxt) as Box<dyn AmbientCtxt + Send + Sync>)
                        .map_clock(|clock| Box::new(clock) as Box<dyn AmbientClock + Send + Sync>)
                        .map_rng(|id_gen| Box::new(id_gen) as Box<dyn AmbientGenId + Send + Sync>);

                    let runtime = Runtime::default()
                        .with_emitter(value.emitter().as_super() as *const _)
                        .with_filter(value.filter().as_super() as *const _)
                        .with_ctxt(value.ctxt().as_super() as *const _)
                        .with_clock(value.clock().as_super() as *const _)
                        .with_rng(value.rng().as_super() as *const _);

                    AmbientSync { value, runtime }
                })
                .ok()?;

            let rt = self.0.get()?;

            Some(
                Runtime::default()
                    .with_emitter(rt.value.emitter().as_any().downcast_ref()?)
                    .with_filter(rt.value.filter().as_any().downcast_ref()?)
                    .with_ctxt(rt.value.ctxt().as_any().downcast_ref()?)
                    .with_clock(rt.value.clock().as_any().downcast_ref()?)
                    .with_rng(rt.value.rng().as_any().downcast_ref()?),
            )
        }

        pub fn get(&self) -> &AmbientRuntime {
            const EMPTY_AMBIENT_RUNTIME: AmbientRuntime = Runtime {
                emitter: &Empty as &(dyn ErasedEmitter + Send + Sync + 'static),
                filter: &Empty as &(dyn ErasedFilter + Send + Sync + 'static),
                ctxt: &Empty as &(dyn ErasedCtxt + Send + Sync + 'static),
                clock: &Empty as &(dyn ErasedClock + Send + Sync + 'static),
                rng: &Empty as &(dyn ErasedRng + Send + Sync + 'static),
            };

            self.0
                .get()
                .map(|rt| unsafe {
                    &*(&rt.runtime as *const AmbientSyncRuntime as *const AmbientRuntime)
                })
                .unwrap_or(&EMPTY_AMBIENT_RUNTIME)
        }
    }

    impl AmbientInternalSlot {
        pub(in crate::runtime) const fn new() -> Self {
            AmbientInternalSlot(AmbientSlot(OnceLock::new()))
        }

        pub fn init<TEmitter, TFilter, TCtxt, TClock, TRng>(
            &self,
            pipeline: Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>,
        ) -> Option<Runtime<&TEmitter, &TFilter, &TCtxt, &TClock, &TRng>>
        where
            TEmitter: InternalEmitter + Send + Sync + 'static,
            TFilter: InternalFilter + Send + Sync + 'static,
            TCtxt: InternalCtxt + Send + Sync + 'static,
            TCtxt::Frame: Send + 'static,
            TClock: InternalClock + Send + Sync + 'static,
            TRng: InternalRng + Send + Sync + 'static,
        {
            self.0.init(pipeline)
        }

        pub fn get(&self) -> &AmbientRuntime {
            self.0.get()
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;

#[cfg(not(feature = "std"))]
mod no_std_support {
    use super::*;

    pub struct AmbientSlot {}

    pub struct AmbientInternalSlot(AmbientSlot);

    impl AmbientSlot {
        pub const fn new() -> Self {
            AmbientSlot {}
        }

        pub fn get(&self) -> &Runtime {
            const EMPTY_AMBIENT_RUNTIME: Runtime = Runtime::new();

            &EMPTY_AMBIENT_RUNTIME
        }
    }

    impl AmbientInternalSlot {
        pub(in crate::runtime) const fn new() -> Self {
            AmbientInternalSlot(AmbientSlot::new())
        }

        pub fn get(&self) -> &Runtime {
            self.0.get()
        }
    }

    pub type AmbientRuntime<'a> = Runtime;
}

#[cfg(not(feature = "std"))]
pub use self::no_std_support::*;
