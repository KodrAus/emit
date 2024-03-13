use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct InternalMetrics {
    pub(crate) otlp_event_discarded: Counter,
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
            otlp_event_discarded: otlp_event_discard,
        } = self;

        [emit::metrics::Metric::new(
            "otlp_event_discard",
            emit::well_known::METRIC_AGG_COUNT,
            otlp_event_discard.sample(),
        )]
        .into_iter()
    }
}
