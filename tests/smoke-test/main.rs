#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::time::Duration;

#[tokio::main]
async fn main() {
    let emitter = emit::setup()
        .to(
            emit_otlp_logs::http("http://localhost:5341/ingest/otlp/v1/logs")
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "smoke-test-rs"
                })
                .spawn(),
        )
        .init();

    in_ctxt(78).await;

    emitter.target().flush(Duration::from_secs(5)).await;
}

#[emit::span("Hello!", a, ax: 13)]
async fn in_ctxt(a: i32) {
    in_ctxt2(5).await;

    emit::info!("an event!");
}

#[emit::span("Hello!", b, bx: 90)]
async fn in_ctxt2(b: i32) {
    // Emit an info event to the global receiver
    emit::warn!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x}!",
        #[emit::fmt("04")]
        x: 15,
    );
}
