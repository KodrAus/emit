use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

macro_rules! metrics {
    (
        $pub_container:ty {
            $field:ident: $internal_container:ident {
                $(
                    $(#[$meta:meta])*
                    $metric:ident: $ty:ident -> $pub_ty:ident,
                )*
            }
        }
    ) => {
        #[derive(Default)]
        pub(crate) struct $internal_container {
            $(
                $(#[$meta])*
                pub(crate) $metric: $ty,
            )*
        }

        impl $internal_container {
            pub fn sample(&self) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
                let $internal_container { $($metric),* } = self;

                [$(
                    emit::metric::Metric::new(
                        env!("CARGO_PKG_NAME"),
                        emit::empty::Empty,
                        stringify!($metric),
                        <$ty>::AGG,
                        $metric.sample(),
                        emit::empty::Empty,
                    ),
                )*]
                .into_iter()
            }
        }

        impl $pub_container {
            $(
                $(#[$meta])*
                pub fn $metric(&self) -> $pub_ty {
                    self.$field.$metric.sample()
                }
            )*
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

metrics!(
    FileSetMetrics {
        metrics: InternalMetrics {
            /**
            Attempting to read the set of log files failed.
            */
            file_set_read_failed: Counter -> usize,
            /**
            Attempting to open a log file failed.
            */
            file_open_failed: Counter -> usize,
            /**
            A new log file was created.
            */
            file_create: Counter -> usize,
            /**
            Attempting to create a new log file failed.
            */
            file_create_failed: Counter -> usize,
            /**
            Attempting to write to a log file failed.
            */
            file_write_failed: Counter -> usize,
            /**
            A log file was deleted.
            */
            file_delete: Counter -> usize,
            /**
            Attempting to delete a log file failed.
            */
            file_delete_failed: Counter -> usize,
            /**
            Attempting to format an event into a batch failed and was discarded.

            This happens before the event is written to any log files.
            */
            event_format_failed: Counter -> usize,
        }
    }
);

/**
Metrics produced by the file writer.

You can enumerate the metrics using the [`emit::metric::Source`] implementation. See [`emit::metric`] for details.
*/
pub struct FileSetMetrics {
    pub(crate) channel_metrics: emit_batcher::ChannelMetrics<crate::EventBatch>,
    pub(crate) metrics: Arc<InternalMetrics>,
}

impl emit::metric::Source for FileSetMetrics {
    fn sample_metrics<S: emit::metric::sampler::Sampler>(&self, sampler: S) {
        self.channel_metrics
            .sample_metrics(emit::metric::sampler::from_fn(|metric| {
                sampler.metric(metric.by_ref().with_module(env!("CARGO_PKG_NAME")));
            }));

        for metric in self.metrics.sample() {
            sampler.metric(metric);
        }
    }
}
