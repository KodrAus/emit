#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[tokio::main]
async fn main() {
    println!(
        "{}",
        emit::tpl!(
            "something went wrong at {id} with {x}!",
            #[emit::fmt(flags: "04")]
            x,
        )
        .render()
        .with_props(emit::props! {
            #[emit::as_debug]
            id: 42,
            x: 15,
        })
    );

    emit::setup()
        .to(emit::target::from_fn(|evt| {
            println!("{:?}", evt);
        }))
        .init();

    in_ctxt(78).await;
}

#[emit::scope("Hello!", a, ax: 13)]
async fn in_ctxt(a: i32) {
    in_ctxt2(5).await;

    emit::info!("an event!");
}

#[emit::scope("Hello!", b, bx: 90)]
async fn in_ctxt2(b: i32) {
    // Emit an info event to the global receiver
    emit::info!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x}!",
        #[emit::fmt("04")]
        x: 15,
    );
}
