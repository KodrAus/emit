use std::{
    io,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use emit::{metric::Source as _, Clock as _, Filter as _};

#[macro_use]
extern crate serde_derive;

#[tokio::main]
async fn main() {
    println!(
        "{}",
        emit::format!("Hello, {x}", #[emit::optional] #[emit::key("x.y")] x: Some("world"))
    );

    let internal = emit::setup().emit_to(emit_term::stdout()).init_internal();

    let mut reporter = emit::metric::Reporter::new();

    // Setup via emit_otlp
    let emitter = emit::setup()
        .emit_to(
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: env!("CARGO_PKG_NAME"),
                    #[emit::key("telemetry.sdk.language")]
                    language: "rust",
                    #[emit::key("telemetry.sdk.name")]
                    sdk: emit_otlp::telemetry_sdk_name(),
                    #[emit::key("telemetry.sdk.version")]
                    version: emit_otlp::telemetry_sdk_version(),
                })
                .scope(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    emit::props! {},
                )
                .logs(
                    emit_otlp::logs_proto(
                        emit_otlp::grpc("http://localhost:4319").headers([("X-ApiKey", "1234")]),
                    )
                    .body(|evt, f| write!(f, "{}", evt.tpl().render(emit::empty::Empty).braced())),
                )
                .traces(
                    emit_otlp::traces_grpc_proto("http://localhost:4319").name(|evt, f| {
                        write!(f, "{}", evt.tpl().render(emit::empty::Empty).braced())
                    }),
                )
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
                .spawn()
                .unwrap()
                .report_to(&mut reporter),
        )
        .and_emit_to(emit_term::stdout())
        .and_emit_to(
            emit::level::min_filter(emit::Level::Warn).wrap_emitter(
                emit_file::set("./target/logs/log.txt")
                    .reuse_files(true)
                    .roll_by_minute()
                    .max_files(6)
                    .spawn()
                    .unwrap(),
            ),
        )
        .init();

    // Setup via opentelemetry
    /*
    let channel = tonic::transport::Channel::from_static("http://localhost:4319")
        .connect()
        .await
        .unwrap();

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_channel(channel.clone()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_channel(channel.clone()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    let emitter = emit::setup()
        .emit_to(emit_opentelemetry::emitter("emit"))
        .map_ctxt(|ctxt| emit_opentelemetry::ctxt("emit", ctxt))
        .and_emit_to(emit_term::stdout())
        .and_emit_to(
            emit::level::min_filter(emit::Level::Warn).wrap_emitter(
                emit_file::set("./target/logs/log.txt")
                    .reuse_files(true)
                    .roll_by_minute()
                    .max_files(6)
                    .spawn()
                    .unwrap(),
            ),
        )
        .init();
    */

    sample_metrics();

    let timer = emit::timer::Timer::start(emit::runtime::shared().clock());

    let _ = in_trace().await;

    emit::emit!(
        extent: timer,
        "{metric_agg} of {metric_name} is {metric_value}",
        event_kind: "metric",
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

    reporter.emit_metrics(&internal.emitter());

    emitter.blocking_flush(Duration::from_secs(60));
    internal.blocking_flush(Duration::from_secs(5));

    //opentelemetry::global::shutdown_logger_provider();
    //opentelemetry::global::shutdown_tracer_provider();
}

#[emit::span("in_trace")]
async fn in_trace() -> Result<(), io::Error> {
    let mut futures = Vec::new();

    for i in 0..2 {
        futures.push(tokio::spawn(
            emit::frame::Frame::current(emit::runtime::shared().ctxt()).in_future(in_ctxt(i)),
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
        span.complete_with(|span| {
            emit::warn!(
                module: span.module(),
                extent: span.extent(),
                props: span.props(),
                "in_ctxt failed with {err}",
            );
        });
    }

    r
}

#[emit::span(arg: span, "in_ctxt2", b, bx: 90)]
async fn in_ctxt2(b: i32) {
    tokio::time::sleep(Duration::from_millis(17)).await;

    span.complete_with(|span| {
        emit::warn!(
            module: span.module(),
            extent: span.extent(),
            props: span.props(),
            "something went wrong at {#[emit::as_debug] id: 42} with {x} and {y: true}!",
            #[emit::fmt(">08")]
            x: 15,
            #[emit::optional]
            z: None::<i32>,
        );
    });
}

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn increment(metric: &AtomicUsize) {
    metric.fetch_add(1, Ordering::Relaxed);
}

fn sample_metrics() {
    let now = emit::runtime::shared().clock().now();

    for (metric_value, metric_agg, metric_name) in [(
        &COUNT,
        emit::well_known::METRIC_AGG_COUNT,
        "smoke_test::count",
    )] {
        emit::emit!(
            extent: now,
            "{metric_agg} of {metric_name} is {metric_value}",
            event_kind: "metric",
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
