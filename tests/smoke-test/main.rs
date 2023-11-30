#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[macro_use]
extern crate serde_derive;

#[tokio::main]
async fn main() {
    let emitter = emit::setup()
        .to(emit_otlp::proto()
            .logs_http("http://localhost:5341/ingest/otlp/v1/logs")
            .traces_http("http://localhost:5341/ingest/otlp/v1/traces")
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: "smoke-test-rs",
            })
            .spawn()
            .unwrap())
        .and_to(emit_term::stdout().plot_metrics_by_count(30))
        .init();

    sample_metrics();

    let _ = in_trace().await;

    emitter.blocking_flush(Duration::from_secs(5));
}

#[emit::with(trace_id: emit::new_trace_id())]
async fn in_trace() -> Result<(), io::Error> {
    for i in 0..100 {
        let _ = in_ctxt(i).await;

        sample_metrics();
    }

    Ok(())
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

        tokio::time::sleep(Duration::from_millis(a as u64)).await;

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

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn increment(metric: &AtomicUsize) {
    metric.fetch_add(1, Ordering::Relaxed);
}

fn sample_metrics() {
    let now = emit::now();

    for (metric, kind, name) in [(
        &COUNT,
        emit::well_known::METRIC_KIND_SUM,
        "smoke_test::count",
    )] {
        emit::emit(&emit::Event::new(
            now,
            emit::tpl!("{metric_kind} of {metric_name} is {metric_value}"),
            emit::metrics::Metric::new(
                emit::key::Key::new(kind),
                emit::key::Key::new(name),
                metric.load(Ordering::Relaxed),
            ),
        ));
    }
}

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}
