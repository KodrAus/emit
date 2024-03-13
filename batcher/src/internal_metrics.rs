use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct InternalMetrics {
    pub(crate) queue_overflow: Counter,
    pub(crate) queue_batch_processed: Counter,
    pub(crate) queue_batch_failed: Counter,
    pub(crate) queue_batch_panicked: Counter,
    pub(crate) queue_batch_retry: Counter,
}

#[derive(Default)]
pub(crate) struct Counter(AtomicUsize);

impl Counter {
    pub const fn new() -> Self {
        Counter(AtomicUsize::new(0))
    }

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
    pub fn sample(&self) -> impl Iterator<Item = emit::metrics::Metric<'static>> + 'static {
        let InternalMetrics {
            queue_overflow,
            queue_batch_processed,
            queue_batch_failed,
            queue_batch_panicked,
            queue_batch_retry,
        } = self;

        [
            emit::metrics::Metric::new(
                "queue_overflow",
                emit::well_known::METRIC_AGG_COUNT,
                queue_overflow.sample(),
            ),
            emit::metrics::Metric::new(
                "queue_batch_processed",
                emit::well_known::METRIC_AGG_COUNT,
                queue_batch_processed.sample(),
            ),
            emit::metrics::Metric::new(
                "queue_batch_failed",
                emit::well_known::METRIC_AGG_COUNT,
                queue_batch_failed.sample(),
            ),
            emit::metrics::Metric::new(
                "queue_batch_panicked",
                emit::well_known::METRIC_AGG_COUNT,
                queue_batch_panicked.sample(),
            ),
            emit::metrics::Metric::new(
                "queue_batch_retry",
                emit::well_known::METRIC_AGG_COUNT,
                queue_batch_retry.sample(),
            ),
        ]
        .into_iter()
    }
}
