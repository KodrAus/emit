use crate::{source::Source, template::Template};

pub struct Record<'a> {
    pub source: Source<'a>,
    pub template: Template<'a>,
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "tracing")]
macro_rules! __private_forward {
    ($($input:tt)*) => {
        antlog_macros_rt::__private_forward_console!($($input)*);
        antlog_macros_rt::__private_forward_tracing!($($input)*);
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "tracing"))]
macro_rules! __private_forward {
    ($($input:tt)*) => {
        antlog_macros_rt::__private_forward_console!($($input)*);
    };
}

pub mod console {
    #[macro_export]
    #[doc(hidden)]
    macro_rules! __private_forward_console {
        ($logger:expr, [$($key:expr),*], [$($value:expr),*], $record:expr) => {{
            println!("kvs (debug): {}", $record.source.render_debug());
            println!("kvs (json):  {}", $record.source.render_json());
            println!("msg:         {}", $record.template.render_template());
            println!("template:    {}", $record.template.render_source($record.source));
        }};
    }
}

#[cfg(feature = "tracing")]
pub mod tracing {
    #[macro_export]
    #[doc(hidden)]
    macro_rules! __private_forward_tracing {
        ($logger:expr, [$($key:expr),*], [$($value:expr),*], $record:expr) => {{
            use antlog_macros_rt::__private::{
                ValueBag,
                tracing::{
                    Callsite,
                    core::{
                        field::{self, FieldSet, Value, DebugValue}, identify_callsite, metadata::Kind,
                        Callsite as TracingCallsite, Level, Metadata, LevelFilter, Event,
                    },
                }
            };

            let level = Level::INFO;

            if level <= LevelFilter::current() {
                static CALLSITE: Callsite = Callsite::new(&META);
                static META: Metadata<'static> = Metadata::new(
                    concat!("event ", file!(), ":", line!()),
                    module_path!(),
                    Level::INFO,
                    Some(file!()),
                    Some(line!()),
                    Some(module_path!()),
                    FieldSet::new(&["msg", $($key),*], identify_callsite!(&CALLSITE)),
                    Kind::EVENT,
                );

                CALLSITE.register();

                if CALLSITE.is_enabled() {
                    let meta = CALLSITE.metadata();
                    let fields = meta.fields();

                    Event::dispatch(meta, &fields.value_set(&[
                        (&fields.field("msg").unwrap(), Some(&field::display($record.template.render_source($record.source)) as &dyn Value)),
                        $(
                            (&fields.field($key).unwrap(), Some(&field::debug($value) as &dyn Value))
                        ),*
                    ]));
                }
            }
        }};
    }

    use std::sync::atomic::{AtomicUsize, Ordering};

    pub use tracing_core as core;

    use tracing_core::{
        callsite, dispatcher, Callsite as TracingCallsite, Interest, Metadata, Once,
    };

    pub type Callsite = MacroCallsite;

    // Inlined from: https://github.com/tokio-rs/tracing/blob/1b5bfa0b996e377bca7cafc70f54f22cfda2b25a/tracing/src/lib.rs#L894-L996
    #[derive(Debug)]
    pub struct MacroCallsite {
        interest: AtomicUsize,
        meta: &'static Metadata<'static>,
        registration: Once,
    }

    impl MacroCallsite {
        pub const fn new(meta: &'static Metadata<'static>) -> Self {
            Self {
                interest: AtomicUsize::new(0),
                meta,
                registration: Once::new(),
            }
        }

        pub fn is_enabled(&self) -> bool {
            let interest = self.interest();
            if interest.is_always() {
                return true;
            }
            if interest.is_never() {
                return false;
            }

            dispatcher::get_default(|current| current.enabled(self.meta))
        }

        #[inline(always)]
        pub fn register(&'static self) {
            self.registration.call_once(|| callsite::register(self));
        }

        #[inline(always)]
        fn interest(&self) -> Interest {
            match self.interest.load(Ordering::Relaxed) {
                0 => Interest::never(),
                2 => Interest::always(),
                _ => Interest::sometimes(),
            }
        }
    }

    impl TracingCallsite for MacroCallsite {
        fn set_interest(&self, interest: Interest) {
            let interest = match () {
                _ if interest.is_never() => 0,
                _ if interest.is_always() => 2,
                _ => 1,
            };
            self.interest.store(interest, Ordering::SeqCst);
        }

        #[inline(always)]
        fn metadata(&self) -> &Metadata<'static> {
            &self.meta
        }
    }
}
