#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate serde_derive;

#[derive(Serialize)]
struct Work {
    id: u64,
    description: String,
}

#[tokio::main]
async fn main() {
    emit::setup().to(emit_term::stdout()).init();

    in_ctxt(78).await;
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
    // Emit an info event to the global receiver
    emit::warn!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x} and {y: true}!",
        x: 15,
    );
}
