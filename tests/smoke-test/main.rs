use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use emit::{Clock as _, Filter as _};

#[macro_use]
extern crate serde_derive;

#[tokio::main]
async fn main() {
    println!(
        "{}",
        emit::format!("Hello, {x}", #[emit::optional] #[emit::key("x.y")] x: Some("world"))
    );

    let internal = emit::setup().emit_to(emit_term::stdout()).init_internal();

    let emitter = emit::setup()
        .emit_to(
            emit_otlp::new()
                .logs(
                    emit_otlp::logs_proto(
                        emit_otlp::grpc("https://localhost:4319").headers([("X-ApiKey", "1234")]),
                    )
                    .body(|evt, f| write!(f, "{}", evt.tpl().render(emit::empty::Empty).braced())),
                )
                .traces(
                    emit_otlp::traces_grpc_proto("https://localhost:4319").name(|evt, f| {
                        write!(f, "{}", evt.tpl().render(emit::empty::Empty).braced())
                    }),
                )
                .metrics(emit_otlp::metrics_grpc_proto("https://localhost:4319"))
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "smoke-test-rs",
                    #[emit::key("telemetry.sdk.language")]
                    language: "rust",
                    #[emit::key("telemetry.sdk.name")]
                    sdk: "emit",
                    #[emit::key("telemetry.sdk.version")]
                    version: "0.1",
                })
                .scope("some-scope", "0.1", emit::props! {})
                .spawn()
                .unwrap(),
        )
        .and_emit_to(emit_term::stdout())
        .and_emit_to(
            emit::level::min_level(emit::Level::Warn).wrap_emitter(
                emit_file::set("./target/logs/log.txt")
                    .reuse_files(true)
                    .roll_by_minute()
                    .max_files(6)
                    .spawn()
                    .unwrap(),
            ),
        )
        .init();

    sample_metrics();

    let timer = emit::timer::Timer::start(emit::runtime::shared());

    let _ = in_trace().await;

    emit::emit!(
        extent: timer,
        "{metric_agg} of {metric_name} is {metric_value}",
        metric_agg: "count",
        metric_name: "smoke_test::sum",
        #[emit::as_value]
        metric_value: [
            1i64,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
        ],
    );

    emit::info!("shutting down");

    emitter.blocking_flush(Duration::from_secs(60));
    internal.blocking_flush(Duration::from_secs(5));
}

#[emit::span("in_trace")]
async fn in_trace() -> Result<(), io::Error> {
    let mut futures = Vec::new();

    for i in 0..100 {
        futures.push(tokio::spawn(
            emit::frame::Frame::current(emit::runtime::shared()).in_future(in_ctxt(i)),
        ));
    }

    for future in futures {
        let _ = future.await;

        sample_metrics();
    }

    Ok(())
}

#[emit::span(arg: span, "in_ctxt", a)]
async fn in_ctxt(a: i32) -> Result<(), io::Error> {
    increment(&COUNT);

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

    if let Err(ref err) = r {
        span.complete(
            |extent| emit::warn!(when: emit::filter::always(), extent, "in_ctxt failed with {err}"),
        );
    }

    r
}

#[emit::span("in_ctxt2", b, bx: 90)]
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
    let now = emit::runtime::shared().now();

    for (metric_value, metric_agg, metric_name) in [(
        &COUNT,
        emit::well_known::METRIC_AGG_COUNT,
        "smoke_test::count",
    )] {
        emit::emit!(
            extent: now,
            "{metric_agg} of {metric_name} is {metric_value}",
            metric_agg,
            metric_name,
            metric_value: metric_value.load(Ordering::Relaxed),
            x: "data",
        );
    }
}

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}
