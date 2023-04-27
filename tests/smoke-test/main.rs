#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
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

    emit::to(emit::target::from_fn(|evt| {
        println!("{:?}", evt);
    }));

    // Emit an info event to the global receiver
    emit::info!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x}!",
        #[emit::fmt(flags: "04")]
        x: 15,
    );
}
