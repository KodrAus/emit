#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::sync::Arc;

fn main() {
    let subscriber = Arc::new(
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .finish(),
    );

    let _ = emit::setup()
        .to(emit_tracing::emitter(subscriber.clone()))
        .map_ctxt(|ctxt| emit_tracing::ctxt(ctxt, subscriber.clone()))
        .init();

    #[emit::span("My span")]
    {
        emit::info!("Hello, world!");
    }
}
