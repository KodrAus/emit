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
    OtlpMetrics {
        metrics: InternalMetrics {
            /**
            An event didn't match a configured OTLP signal and the logs signal is not configured, so it was discarded.
            */
            event_discarded: Counter -> usize,
            /**
            A connection to a remote OTLP receiver was established successfully.
            */
            transport_conn_established: Counter -> usize,
            /**
            A connection to a remote OTLP receiver could not be established.
            */
            transport_conn_failed: Counter -> usize,
            /**
            A TLS handshake with a remote OTLP receiver was made successfully.
            */
            transport_conn_tls_handshake: Counter -> usize,
            /**
            A TLS handshake with a remote OTLP receiver could not be made.
            */
            transport_conn_tls_failed: Counter -> usize,
            /**
            A request was sent successfully.
            */
            transport_request_sent: Counter -> usize,
            /**
            A request could not be sent.
            */
            transport_request_failed: Counter -> usize,
            /**
            The body of a request was compressed using gzip.
            */
            transport_request_compress_gzip: Counter -> usize,
            /**
            A gRPC export request was sent and responded with a successful status code.
            */
            http_batch_sent: Counter -> usize,
            /**
            A gRPC export request was sent but responded with a failed status code.
            */
            http_batch_failed: Counter -> usize,
            /**
            A HTTP export request was sent and responded with a successful status code.
            */
            grpc_batch_sent: Counter -> usize,
            /**
            A HTTP export request was sent but responded with a failed status code.
            */
            grpc_batch_failed: Counter -> usize,
        }
    }
);

/**
Metrics produced by an OTLP emitter itself.

This type doesn't collect any OTLP metrics you emit, it includes metrics about the OTLP emitter's own activity.

You can enumerate the metrics using the [`emit::metric::Source`] implementation. See [`emit::metric`] for details.
*/
pub struct OtlpMetrics {
    pub(crate) channel_metrics: emit_batcher::ChannelMetrics<crate::client::Channel>,
    pub(crate) metrics: Arc<InternalMetrics>,
}

impl emit::metric::Source for OtlpMetrics {
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
