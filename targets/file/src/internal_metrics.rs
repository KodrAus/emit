use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! metrics {
    ($container:ident {
        $(
            $name:ident: $ty:ty,
        )*
    }) => {
        #[derive(Default)]
        pub(crate) struct $container {
            $(
                pub(crate) $name: $ty
            ),*
        }

        impl $container {
            pub fn sample(
                &self,
            ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
                let $container {
                    $(
                        $name
                    ),*
                } = self;

                [
                    $(
                        emit::metric::Metric::new(
                            env!("CARGO_PKG_NAME"),
                            emit::empty::Empty,
                            stringify!($name),
                            emit::well_known::METRIC_AGG_COUNT,
                            $name.sample(),
                            emit::empty::Empty,
                        )
                    ),*
                ]
                .into_iter()
            }
        }
    };
}

metrics!(InternalMetrics {
    file_set_read_failed: Counter,
    file_open_failed: Counter,
    file_create: Counter,
    file_create_failed: Counter,
    file_write_failed: Counter,
    file_delete: Counter,
    file_delete_failed: Counter,
});

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
