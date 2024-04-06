use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! metrics {
    ($container:ident {
        $($name:ident: $ty:ty,)*
    }) => {
        #[derive(Default)]
        pub(crate) struct $container { $(pub(crate) $name: $ty),* }

        impl $container {
            pub fn sample(&self) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
                let $container { $($name),* } = self;

                [$(
                    emit::metric::Metric::new(
                        env!("CARGO_PKG_NAME"),
                        emit::empty::Empty,
                        stringify!($name),
                        <$ty>::AGG,
                        $name.sample(),
                        emit::empty::Empty,
                    )
                ),*]
                .into_iter()
            }
        }
    };
}

#[derive(Default)]
pub(crate) struct Counter(AtomicUsize);

impl Counter {
    const AGG: &'static str = emit::well_known::METRIC_AGG_COUNT;

    pub fn increment(&self) {
        self.increment_by(1);
    }

    pub fn increment_by(&self, by: usize) {
        self.0.fetch_add(by, Ordering::Relaxed);
    }

    pub fn sample(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

metrics!(InternalMetrics {
    otlp_event_discarded: Counter,
    otlp_http_conn_established: Counter,
    otlp_http_conn_failed: Counter,
    otlp_http_conn_tls_handshake: Counter,
    otlp_http_conn_tls_failed: Counter,
    otlp_http_request_sent: Counter,
    otlp_http_request_failed: Counter,
    otlp_http_compress_gzip: Counter,
});
