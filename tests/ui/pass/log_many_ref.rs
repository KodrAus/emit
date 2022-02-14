#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    tracing_subscriber::fmt().init();

    fn call(v: &&&str) {
        emit::info!("Logging with a deeply nested {value: ***v}");
        emit::info!("Logging with a deeply nested {#[emit::as_display] value: v}");
    }
}
