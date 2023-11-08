#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use emit::Props;

#[macro_use]
extern crate serde_derive;

struct DeltaCount {
    last: std::sync::Mutex<(Option<emit::timestamp::Timestamp>, usize)>,
    value: AtomicUsize,
}

static COUNT: emit::Metric<'static, DeltaCount> = emit::Metric::new(
    emit::Key::new("smoke_test::count"),
    emit::metrics::MetricKind::Counter,
    DeltaCount {
        last: std::sync::Mutex::new((None, 0)),
        value: AtomicUsize::new(0),
    },
);

fn increment(metric: &emit::Metric<DeltaCount>) {
    metric.value().value.fetch_add(1, Ordering::Relaxed);
}

fn flush_metrics<'a>(metrics: impl IntoIterator<Item = &'a emit::Metric<'a, DeltaCount>> + 'a) {
    let now = emit::now();

    for metric in metrics {
        let mut start = None;
        let delta = metric.sample(|_, value| {
            let mut previous = value.last.lock().unwrap();
            let current = value.value.load(Ordering::Relaxed);

            start = previous.0;
            let delta = current.saturating_sub(previous.1);

            previous.0 = now;
            previous.1 = current;

            (emit::metrics::MetricKind::Counter, delta)
        });

        emit::emit(&emit::Event::new(
            start..now,
            emit::tpl!("{metric_name} read {metric_value} with {ordering}"),
            delta.chain(emit::props! {
                ordering: "relaxed"
            }),
        ));
    }
}

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}

#[tokio::main]
async fn main() {
    let emitter = emit::setup()
        .to(
            emit_otlp::logs::http_proto("http://localhost:5341/ingest/otlp/v1/logs")
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "smoke-test-rs",
                })
                .spawn()
                .unwrap(),
        )
        .and_to(emit_term::stdout())
        .init();

    for i in 0..9 {
        let _ = in_ctxt(i).await;
    }

    flush_metrics([&COUNT]);

    for i in 0..7 {
        let _ = in_ctxt(i).await;
    }

    flush_metrics([&COUNT]);

    emitter.blocking_flush(Duration::from_secs(5));
}

#[emit::with(span_id: emit::new_span_id(), span_parent: emit::current_span_id(), a)]
async fn in_ctxt(a: i32) -> Result<(), io::Error> {
    increment(&COUNT);

    let extent = emit::start_timer();

    let r = async {
        in_ctxt2(5).await;

        let work = Work {
            id: 42,
            description: "Some very important business".to_owned(),
        };

        emit::info!("working on {#[emit::as_serde] work}");

        tokio::time::sleep(Duration::from_millis(100)).await;

        if a % 2 == 0 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "`a` is odd"))
        }
    }
    .await;

    match r {
        Ok(_) => emit::info!(extent, "in_ctxt finished"),
        Err(ref err) => emit::warn!(extent, "in_ctxt failed with {err}"),
    }

    r
}

#[emit::with(b, bx: 90)]
async fn in_ctxt2(b: i32) {
    emit::warn!(
        "something went wrong at {#[emit::as_debug] id: 42} with {x} and {y: true}!",
        #[emit::fmt(">08")]
        x: 15,
        #[emit::optional]
        z: None::<i32>,
    );
}
