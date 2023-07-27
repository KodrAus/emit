#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::time::Duration;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}

#[tokio::main]
async fn main() {
    let emitter = emit::setup()
        .to(emit_term::stdout())
        .and_to(
            emit_otlp::logs::http("http://localhost:5341/ingest/otlp/v1/logs")
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "smoke-test-rs",
                })
                .spawn(),
        )
        .init();

    in_ctxt(78).await;

    emitter.blocking_flush(Duration::from_secs(5));
}

#[emit::with(a, ax: 13)]
async fn in_ctxt(a: i32) {
    let ts = emit::__private::__span_start();

    in_ctxt2(5).await;

    let work = Work {
        id: 42,
        description: "Some very important business".to_owned(),
    };

    tokio::time::sleep(Duration::from_secs(1)).await;

    let ts = emit::__private::__span_end(ts);

    emit::info!(ts, "working on {#[emit::as_serde] work}");
}

#[emit::with(b, bx: 90)]
async fn in_ctxt2(b: i32) {
    emit::warn!(
        "something went wrong at {#[emit::as_debug] id: 42} with {x} and {y: true}!",
        #[emit::fmt(">08")]
        x: 15,
    );
}
