#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::{io, time::Duration};

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
        .to(
            emit_otlp::logs::http_proto("http://localhost:5341/ingest/otlp/v1/logs")
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "smoke-test-rs",
                })
                .spawn(),
        )
        .and_to(emit_term::stdout())
        .init();

    for i in 0..100 {
        let _ = in_ctxt(i).await;
    }

    emitter.blocking_flush(Duration::from_secs(5));
}

#[emit::with(span_id: emit::new_span_id(), span_parent: emit::current_span_id(), a)]
async fn in_ctxt(a: i32) -> Result<(), io::Error> {
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
