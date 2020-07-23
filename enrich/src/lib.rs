/*!
Enriched logging.

This crate allows you to enrich log records within a scope with a collection of properties.
It's compatible with `log`.

- Call `sync::logger()` to create a scope.
- Call `future::logger()` to create a scope that works with `futures`.
*/

#![cfg_attr(test, feature(test))]

#[macro_use]
extern crate pin_project;

mod ctxt;
mod properties;

use std::sync::Arc;

use self::ctxt::{Ctxt, LocalCtxt, Scope, SharedCtxt};
use self::properties::Properties;
use stdlog::kv::value::ToValue;

fn current_logger() -> Logger {
    BuilderInner::default().get()
}

#[derive(Default)]
struct BuilderCtxt {
    properties: Properties,
}

#[derive(Default)]
struct BuilderInner {
    ctxt: Option<BuilderCtxt>,
}

struct Logger {
    ctxt: Option<LocalCtxt>,
}

impl BuilderInner {
    fn into_logger(self) -> Logger {
        // Capture the current context
        // Each logger keeps a copy of the context it was created in so it can be shared
        // This context is set by other loggers calling `.scope()`
        let ctxt = if let Some(ctxt) = self.ctxt {
            SharedCtxt::scope_current(|mut scope| {
                Some(Arc::new(Ctxt::from_scope(ctxt.properties, &mut scope)))
            })
        } else {
            None
        };

        Logger {
            ctxt: ctxt.map(|local| LocalCtxt::new(local)),
        }
    }

    fn enrich<V>(mut self, k: &'static str, v: V) -> Self
    where
        V: ToValue,
    {
        let ctxt = self.ctxt.get_or_insert_with(|| BuilderCtxt {
            properties: Default::default(),
        });

        let v = sval::value::OwnedValue::collect(v.to_value());
        ctxt.properties_mut().insert(k, v);

        self
    }

    fn get(self) -> Logger {
        self.into_logger()
    }
}

impl Logger {
    fn scope<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Scope) -> R,
    {
        // Set the current shared log context
        // This makes the context available to other loggers on this thread
        // within the `scope` function
        if let Some(ref mut ctxt) = self.ctxt {
            SharedCtxt::scope(ctxt, f)
        } else {
            SharedCtxt::scope_current(f)
        }
    }
}

impl BuilderCtxt {
    fn properties_mut(&mut self) -> &mut Properties {
        &mut self.properties
    }
}

pub mod future {
    use super::{BuilderCtxt, BuilderInner, Logger};

    use std::pin::Pin;

    use futures::{
        future::Future,
        task::{Context, Poll},
    };
    use stdlog::kv::value::ToValue;

    /**
    A builder for a logging scope.

    Call `.enrich` to add properties and `.scope` to create a scope containing the enriched properties.
    */
    pub fn logger() -> Builder {
        // Ensure that user created scopes always have properties
        // This ensures they include properties when sent to other threads
        Builder {
            inner: BuilderInner {
                ctxt: Some(BuilderCtxt::default()),
            },
        }
    }

    /**
    A builder for a logging scope.

    Call `.enrich` to add properties and `.scope` to create a scope containing the enriched properties.
    */
    pub struct Builder {
        inner: BuilderInner,
    }

    impl Builder {
        /**
        Set a property on this logger.

        If this logger is inside another scope, and that scope has a property with the same name, then the previous value will be overriden.
        */
        pub fn enrich<V>(mut self, k: &'static str, v: V) -> Self
        where
            V: ToValue,
        {
            self.inner = self.inner.enrich(k, v);
            self
        }

        /**
        Create a scope where the enriched properties will be logged.

        Scopes are stacked, so if this logger is inside another scope, the scope created here will contain all of its properties too.
        The returned `ScopeFuture` will retain all of these properties, even if it's sent across threads.

        **NOTE:** Enriched properties aren't visible on threads spawned within a scope unless a child scope is sent to them.
        */
        pub async fn scope<F>(self, f: F) -> F::Output
        where
            F: Future,
        {
            #[pin_project]
            pub struct ScopeFuture<TFuture> {
                logger: Logger,
                #[pin]
                inner: TFuture,
            }

            impl<TFuture> Future for ScopeFuture<TFuture>
            where
                TFuture: Future,
            {
                type Output = TFuture::Output;

                fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                    let mut this = self.project();

                    let logger = this.logger;
                    let inner = this.inner.as_mut();

                    logger.scope(|_| inner.poll(cx))
                }
            }

            let logger = self.inner.into_logger();

            let f = ScopeFuture { logger, inner: f };

            f.await
        }
    }
}

pub mod sync {
    /*!
    Property enrichment for futures.

    Scopes created by loggers in this module are _futures_.
    They maintain their context wherever the future is executed, including other threads.
    */

    use stdlog::kv::value::ToValue;

    use super::BuilderInner;

    /**
    A builder for a logging scope.

    Call `.enrich` to add properties and `.scope` to create a scope containing the enriched properties.
    */
    pub fn logger() -> Builder {
        Builder {
            inner: Default::default(),
        }
    }

    /**
    A builder for a logging scope.

    Call `.enrich` to add properties and `.scope` to create a scope containing the enriched properties.
    */
    pub struct Builder {
        inner: BuilderInner,
    }

    impl Builder {
        /**
        Set a property on this logger.

        If this logger is inside another scope, and that scope has a property with the same name, then the previous value will be overriden.
        */
        pub fn enrich<V>(mut self, k: &'static str, v: V) -> Self
        where
            V: ToValue,
        {
            self.inner = self.inner.enrich(k, v);
            self
        }

        /**
        Create a scope where the enriched properties will be logged.

        Scopes are stacked, so if this logger is inside another scope, the scope created here will contain all of its properties too.

        **NOTE:** Enriched properties aren't visible on threads spawned within a scope unless a child `ScopeFuture` is sent to them.
        */
        pub fn scope<F, R>(self, f: F) -> R
        where
            F: FnOnce() -> R,
        {
            let mut logger = self.inner.into_logger();

            logger.scope(|_| f())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate test;

    use std::{panic, thread};

    use self::test::Bencher;
    use futures::executor;
    use serde_json::{json, Value};
    use stdlog::{Level, Record, RecordBuilder};

    use sval::value;

    use crate::ctxt::Ctxt;

    #[allow(unused)]
    pub(crate) struct Log<'a, 'b> {
        ctxt: Option<&'a Ctxt>,
        record: Record<'b>,
    }

    impl<'a, 'b> Log<'a, 'b> {
        #[cfg(test)]
        pub(crate) fn new(ctxt: Option<&'a Ctxt>, record: Record<'b>) -> Self {
            Log { ctxt, record }
        }
    }

    impl<'a, 'b> value::Value for Log<'a, 'b> {
        fn stream(&self, stream: &mut value::Stream) -> value::Result {
            stream.map_begin(None)?;

            stream.map_key("msg")?;
            stream.map_value(self.record.args())?;

            stream.map_key("ctxt")?;
            stream.map_value(self.ctxt.as_ref().map(|ctxt| ctxt.properties()))?;

            stream.map_end()
        }
    }

    impl Scope {
        fn log<'b, 'c>(&'b mut self, record: Record<'c>) -> Log<'b, 'c> {
            Log::new(self.current(), record)
        }

        fn log_value<'b>(&mut self, record: Record<'b>) -> Value {
            serde_json::to_value(&sval::serde::to_serialize(self.log(record))).unwrap()
        }

        fn log_string<'b>(&mut self, record: Record<'b>) -> String {
            serde_json::to_string(&sval::serde::to_serialize(self.log(record))).unwrap()
        }
    }

    macro_rules! record {
        () => {
            RecordBuilder::new()
                .args(format_args!("Hi {}!", "user"))
                .level(Level::Info)
                .build()
        };
    }

    fn log_value() -> Value {
        current_logger().scope(|ctxt| ctxt.log_value(record!()))
    }

    fn log_string() -> String {
        current_logger().scope(|ctxt| ctxt.log_string(record!()))
    }

    fn assert_log(expected: Value) {
        for _ in 0..5 {
            let log = log_value();
            assert_eq!(expected, log);
        }
    }

    #[test]
    fn basic() {
        let log = log_value();

        let expected = json!({
            "msg": "Hi user!",
            "ctxt": Value::Null
        });

        assert_eq!(expected, log);
    }

    #[test]
    fn enriched_empty() {
        sync::logger().scope(|| {
            assert_log(json!({
                "msg": "Hi user!",
                "ctxt": Value::Null
            }));
        });
    }

    #[test]
    fn enriched_basic() {
        sync::logger()
            .enrich("correlation", "An Id")
            .enrich("service", "Banana")
            .scope(|| {
                assert_log(json!({
                    "msg": "Hi user!",
                    "ctxt": {
                        "correlation": "An Id",
                        "service": "Banana"
                    }
                }));
            });
    }

    #[test]
    fn enriched_nested() {
        sync::logger()
            .enrich("correlation", "An Id")
            .enrich("service", "Banana")
            .scope(|| {
                sync::logger().enrich("service", "Mandarin").scope(|| {
                    assert_log(json!({
                        "msg": "Hi user!",
                        "ctxt": {
                            "correlation": "An Id",
                            "service": "Mandarin"
                        }
                    }));
                });

                sync::logger().enrich("service", "Onion").scope(|| {
                    assert_log(json!({
                        "msg": "Hi user!",
                        "ctxt": {
                            "correlation": "An Id",
                            "service": "Onion"
                        }
                    }));
                });
            });
    }

    #[test]
    fn enriched_panic() {
        sync::logger()
            .enrich("correlation", "An Id")
            .enrich("service", "Banana")
            .scope(|| {
                let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    sync::logger().enrich("service", "Mandarin").scope(|| {
                        sync::logger().enrich("service", "Onion").scope(|| {
                            assert_log(json!({
                                "msg": "Hi user!",
                                "ctxt": {
                                    "correlation": "An Id",
                                    "service": "Onion"
                                }
                            }));

                            panic!("panic to catch_unwind");
                        });
                    });
                }));

                assert_log(json!({
                    "msg": "Hi user!",
                    "ctxt": {
                        "correlation": "An Id",
                        "service": "Banana"
                    }
                }));
            });
    }

    #[test]
    fn enriched_multiple_threads() {
        let f = future::logger()
            .enrich("correlation", "An Id")
            .enrich("operation", "Logging")
            .enrich("service", "Banana")
            .scope(
                future::logger()
                    .enrich("correlation", "Another Id")
                    .scope(async {
                        assert_log(json!({
                            "msg": "Hi user!",
                            "ctxt": {
                                "correlation": "Another Id",
                                "context": "bg-thread",
                                "operation": "Logging",
                                "service": "Banana"
                            }
                        }));
                    }),
            );

        thread::spawn(move || {
            let f = future::logger()
                .enrich("context", "bg-thread")
                .enrich("service", "Mandarin")
                .scope(f);

            executor::block_on(f)
        })
        .join()
        .unwrap();
    }

    #[bench]
    fn create_scope_empty(b: &mut Bencher) {
        b.iter(|| current_logger())
    }

    #[bench]
    fn serialize_log_empty(b: &mut Bencher) {
        b.iter(|| log_string());
    }

    #[bench]
    fn serialize_log_1(b: &mut Bencher) {
        sync::logger().enrich("correlation", "An Id").scope(|| {
            b.iter(|| log_string());
        });
    }

    #[bench]
    fn create_scope_1(b: &mut Bencher) {
        b.iter(|| sync::logger().enrich("correlation", "An Id").scope(|| ()))
    }

    #[bench]
    fn create_scope_1_nested(b: &mut Bencher) {
        sync::logger().enrich("correlation", "An Id").scope(|| {
            b.iter(|| sync::logger().enrich("correlation", "An Id").scope(|| ()));
        });
    }

    #[bench]
    fn create_scope_1_nested_2(b: &mut Bencher) {
        sync::logger().enrich("correlation", "An Id").scope(|| {
            sync::logger().enrich("correlation", "An Id").scope(|| {
                b.iter(|| sync::logger().enrich("correlation", "An Id").scope(|| ()));
            });
        });
    }
}
