use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct InternalMetrics {
    pub(crate) otlp_event_discarded: Counter,
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
        } = self;

        [emit::metric::Metric::new(
            "emit_otlp",
            emit::empty::Empty,
            stringify!(otlp_event_discarded),
            emit::well_known::METRIC_AGG_COUNT,
            otlp_event_discarded.sample(),
            emit::empty::Empty,
        )]
        .into_iter()
    }
}
