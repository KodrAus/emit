#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    // Emit an info event to the global receiver
    emit::info!("something went wrong at {id: 42}");
}
