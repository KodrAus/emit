use std::sync::atomic::{AtomicUsize, Ordering};

use emit::Clock;

macro_rules! increment {
    ($metric:path) => {
        increment!($metric, 1);
    };
    ($metic:path, $by:expr) => {
        $metic.fetch_add($by, std::sync::atomic::Ordering::Relaxed);
    };
}

pub(crate) static INTERNAL_METRICS: InternalMetrics = InternalMetrics {
    log_batches_sent: AtomicUsize::new(0),
};

pub(crate) struct InternalMetrics {
    pub(crate) log_batches_sent: AtomicUsize,
}

impl InternalMetrics {
    pub(crate) fn emit(&self) {
        let InternalMetrics {
            ref log_batches_sent,
        } = self;

        let rt = emit::runtime::internal();

        let extent = rt.now();

        emit::info!(
            rt,
            extent,
            "{metric_value} {signal} batches sent",
            metric_name: "batches_sent",
            metric_agg: "count",
            metric_value: log_batches_sent.load(Ordering::Relaxed),
            signal: "logs"
        );
    }
}
