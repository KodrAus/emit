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
    println!("{}", emit::format!("Hello, {x}", x: "world"));

    let emitter = emit::setup()
        .to(emit_otlp::proto()
            .logs(
                emit_otlp::logs_http("http://localhost:4318/v1/logs")
                    .body(|evt, f| write!(f, "{}", evt.tpl().braced())),
            )
            .traces(
                emit_otlp::traces_http("http://localhost:4318/v1/traces")
                    .name(|evt, f| write!(f, "{}", evt.tpl().braced())),
            )
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: "smoke-test-rs",
                #[emit::key("telemetry.sdk.language")]
                language: "rust",
                #[emit::key("telemetry.sdk.name")]
                sdk: "emit",
                #[emit::key("telemetry.sdk.version")]
                version: "0.1"
            })
            .scope("some-scope", "0.1", emit::props! {})
            .spawn()
            .unwrap())
        //.and_to(emit_metrics::plot_metrics_by_count(30, emit_term::stdout()))
        .and_to(
            emit_file::set("./target/logs/log.txt")
                .reuse_files(true)
                .roll_by_minute()
                .max_files(6)
                .spawn()
                .unwrap(),
        )
        .init();

    //emit::setup().to(emit_term::stdout()).init_internal();

    sample_metrics();

    let _ = in_trace().await;

    emitter.blocking_flush(Duration::from_secs(5));
}

#[emit::in_ctxt(trace_id: emit::new_trace_id())]
async fn in_trace() -> Result<(), io::Error> {
    let mut futures = Vec::new();

    for i in 0..100 {
        futures.push(tokio::spawn(emit::current_ctxt().with_future(in_ctxt(i))));
    }

    for future in futures {
        let _ = future.await;

        sample_metrics();
    }

    Ok(())
}

#[emit::in_ctxt(span_id: emit::new_span_id(), span_parent: emit::current_span_id(), a)]
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

#[emit::in_ctxt(b, bx: 90)]
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

    for (metric_value, metric_kind, metric_name) in [(
        &COUNT,
        emit::well_known::METRIC_KIND_SUM,
        "smoke_test::count",
    )] {
        emit::emit(&emit::Event::new(
            now,
            emit::tpl!("{metric_kind} of {metric_name} is {metric_value}"),
            emit::props! {
                metric_kind,
                metric_name,
                metric_value: metric_value.load(Ordering::Relaxed),
            },
        ));
    }
}

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}
