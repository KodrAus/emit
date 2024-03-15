use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(crate) struct InternalMetrics {
    pub(crate) file_set_read_failed: Counter,
    pub(crate) file_open_failed: Counter,
    pub(crate) file_create: Counter,
    pub(crate) file_create_failed: Counter,
    pub(crate) file_write_failed: Counter,
    pub(crate) file_delete: Counter,
    pub(crate) file_delete_failed: Counter,
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
    pub fn sample(&self) -> impl Iterator<Item = emit::metrics::Metric<'static, emit::empty::Empty>> + 'static {
        let InternalMetrics {
            file_set_read_failed,
            file_open_failed,
            file_create,
            file_create_failed,
            file_write_failed,
            file_delete,
            file_delete_failed,
        } = self;

        [
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_set_read_failed),
                emit::well_known::METRIC_AGG_COUNT,
                file_set_read_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_open_failed),
                emit::well_known::METRIC_AGG_COUNT,
                file_open_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_create),
                emit::well_known::METRIC_AGG_COUNT,
                file_create.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_create_failed),
                emit::well_known::METRIC_AGG_COUNT,
                file_create_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_write_failed),
                emit::well_known::METRIC_AGG_COUNT,
                file_write_failed.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_delete),
                emit::well_known::METRIC_AGG_COUNT,
                file_delete.sample(),
                emit::empty::Empty,
            ),
            emit::metrics::Metric::new(
                "emit_file",
                emit::empty::Empty,
                stringify!(file_delete_failed),
                emit::well_known::METRIC_AGG_COUNT,
                file_delete_failed.sample(),
                emit::empty::Empty,
            ),
        ]
        .into_iter()
    }
}
