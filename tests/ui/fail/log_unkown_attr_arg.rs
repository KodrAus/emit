#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    tracing_subscriber::fmt().init();

    let v = "Some data";

    emit::info!("Logging with a deeply nested {#[emit::as_display(not_an_arg: false)] value: v}");
}
