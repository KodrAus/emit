#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    // Emit an info event to the global receiver
    emit::info!("something went wrong at {#[emit::as_debug] id: 42} with {x}", #[emit::fmt(flags: "?>08b")] x: 15);
}
