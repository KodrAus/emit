use crate::{empty::Empty, time::Time, Ctxt, Event, Filter, Props, Target, Timestamp};

#[cfg(feature = "std")]
use crate::{ctxt::ErasedCtxt, filter::ErasedFilter, target::ErasedTarget, time::ErasedTime};

#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(feature = "std")]
static AMBIENT: OnceLock<
    Ambient<
        Box<dyn ErasedTarget + Send + Sync>,
        Box<dyn ErasedFilter + Send + Sync>,
        Box<dyn ErasedCtxt + Send + Sync>,
        Box<dyn ErasedTime + Send + Sync>,
    >,
> = OnceLock::new();

pub(crate) fn get() -> Option<&'static (impl Target + Filter + Ctxt + Time)> {
    #[cfg(feature = "std")]
    {
        AMBIENT.get()
    }
    #[cfg(not(feature = "std"))]
    {
        None::<&'static Ambient>
    }
}

struct Ambient<TTarget = Empty, TFilter = Empty, TCtxt = Empty, TTime = Empty> {
    target: TTarget,
    filter: TFilter,
    ctxt: TCtxt,
    time: TTime,
}

impl<TTarget: Target, TFilter, TCtxt, TTime> Target for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.target.emit_event(evt)
    }
}

impl<TTarget, TFilter: Filter, TCtxt, TTime> Filter for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn matches_event<P: Props>(&self, evt: &Event<P>) -> bool {
        self.filter.matches_event(evt)
    }
}

impl<TTarget, TFilter, TCtxt: Ctxt, TTime> Ctxt for Ambient<TTarget, TFilter, TCtxt, TTime> {
    type Props = TCtxt::Props;
    type Span = TCtxt::Span;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.ctxt.with_props(with)
    }

    fn open<P: Props>(&self, props: P) -> Self::Span {
        self.ctxt.open(props)
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

impl<TTarget, TFilter, TCtxt, TTime: Time> Time for Ambient<TTarget, TFilter, TCtxt, TTime> {
    fn timestamp(&self) -> Option<Timestamp> {
        self.time.timestamp()
    }
}

#[cfg(feature = "std")]
mod std_support {
    use core::marker::PhantomData;

    use super::*;

    use crate::{ctxt::thread_local::ThreadLocalCtxt, time::SystemClock};

    pub struct Setup<TTarget = Empty, TFilter = Empty, TCtxt = ThreadLocalCtxt, TTime = SystemClock> {
        target: TTarget,
        filter: TFilter,
        ctxt: TCtxt,
        time: TTime,
    }

    impl Default for Setup {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Setup {
        pub fn new() -> Self {
            Setup {
                target: Empty,
                filter: Empty,
                ctxt: ThreadLocalCtxt,
                time: SystemClock,
            }
        }
    }

    impl<TTarget, TFilter, TCtxt, TTime> Setup<TTarget, TFilter, TCtxt, TTime> {
        pub fn to<UTarget>(self, target: UTarget) -> Setup<UTarget, TFilter, TCtxt, TTime> {
            Setup {
                target,
                filter: self.filter,
                ctxt: self.ctxt,
                time: self.time,
            }
        }

        pub fn with<UCtxt>(self, ctxt: UCtxt) -> Setup<TTarget, TFilter, UCtxt, TTime> {
            Setup {
                target: self.target,
                filter: self.filter,
                ctxt,
                time: self.time,
            }
        }
    }

    impl<
            TTarget: Target + Send + Sync + 'static,
            TFilter: Filter + Send + Sync + 'static,
            TCtxt: Ctxt + Send + Sync + 'static,
            TTime: Time + Send + Sync + 'static,
        > Setup<TTarget, TFilter, TCtxt, TTime>
    where
        TCtxt::Span: Send + 'static,
    {
        pub fn init(self) -> Init<TTarget, TFilter, TCtxt, TTime> {
            let target = Box::new(self.target);
            let filter = Box::new(self.filter);
            let ctxt = Box::new(self.ctxt);
            let time = Box::new(self.time);

            let _ = AMBIENT.set(Ambient {
                target,
                filter,
                ctxt,
                time,
            });

            Init(PhantomData)
        }
    }

    pub struct Init<TTarget, TFilter, TCtxt, TTime>(
        PhantomData<Setup<TTarget, TFilter, TCtxt, TTime>>,
    );

    impl<TTarget: 'static, TFilter: 'static, TCtxt: 'static, TTime: 'static>
        Init<TTarget, TFilter, TCtxt, TTime>
    {
        pub fn target(&self) -> &TTarget {
            unsafe { &*(&*AMBIENT.get().unwrap().target as *const _ as *const TTarget) }
        }

        pub fn filter(&self) -> &TFilter {
            unsafe { &*(&*AMBIENT.get().unwrap().filter as *const _ as *const TFilter) }
        }

        pub fn ctxt(&self) -> &TCtxt {
            unsafe { &*(&*AMBIENT.get().unwrap().ctxt as *const _ as *const TCtxt) }
        }

        pub fn time(&self) -> &TTime {
            unsafe { &*(&*AMBIENT.get().unwrap().time as *const _ as *const TTime) }
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
