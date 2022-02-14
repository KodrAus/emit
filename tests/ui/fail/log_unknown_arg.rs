#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    tracing_subscriber::fmt().init();

    let v = "Some data";

    emit::info!(not_an_arg: false, "Logging with a deeply nested {value: v}");
}
