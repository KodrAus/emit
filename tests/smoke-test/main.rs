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
        .to(
            emit_otlp_logs::http("http://localhost:5341/ingest/otlp/v1/logs")
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

#[emit::span("Hello!", a, ax: 13)]
async fn in_ctxt(a: i32) {
    in_ctxt2(5).await;

    let work = Work {
        id: 42,
        description: "Some very important business".to_owned(),
    };

    emit::info!("working on {#[emit::as_serde] work}");
}

#[emit::span("Hello!", b, bx: 90)]
async fn in_ctxt2(b: i32) {
    emit::warn!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x} and {y: true}!",
        #[emit::fmt(">08")]
        x: 15,
    );
}
