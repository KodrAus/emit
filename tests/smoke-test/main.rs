#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::to(emit::target::from_fn(|evt| {
        println!("{}", evt.msg());
    }));

    // Emit an info event to the global receiver
    emit::info!(
        "something went wrong at {#[emit::as_debug] id: 42} with {x}!",
        #[emit::fmt(flags: "04")]
        x: 15,
    );
}
