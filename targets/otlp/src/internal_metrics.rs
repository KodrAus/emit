use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct InternalMetrics {
    pub(crate) otlp_event_discarded: Counter,
    pub(crate) otlp_http_conn_established: Counter,
    pub(crate) otlp_http_conn_failed: Counter,
    pub(crate) otlp_http_conn_tls_handshake: Counter,
    pub(crate) otlp_http_conn_tls_failed: Counter,
    pub(crate) otlp_http_request_sent: Counter,
    pub(crate) otlp_http_request_failed: Counter,
    pub(crate) otlp_http_compress_gzip: Counter,
}

#[derive(Default)]
pub(crate) struct Counter(AtomicUsize);

impl Counter {
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

impl InternalMetrics {
    pub fn sample(
        &self,
    ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
        let InternalMetrics {
            otlp_event_discarded,
            otlp_http_conn_established,
            otlp_http_conn_failed,
            otlp_http_conn_tls_handshake,
            otlp_http_conn_tls_failed,
            otlp_http_request_sent,
            otlp_http_request_failed,
            otlp_http_compress_gzip,
        } = self;

        [
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_event_discarded),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_event_discarded.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_conn_established),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_conn_established.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_conn_failed),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_conn_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_conn_tls_handshake),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_conn_tls_handshake.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_conn_tls_failed),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_conn_tls_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_request_sent),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_request_sent.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_request_failed),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_request_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metric::Metric::new(
                "emit_otlp",
                emit::empty::Empty,
                stringify!(otlp_http_compress_gzip),
                emit::well_known::METRIC_AGG_COUNT,
                otlp_http_compress_gzip.sample(),
                emit::empty::Empty,
            ),
        ]
        .into_iter()
    }
}
