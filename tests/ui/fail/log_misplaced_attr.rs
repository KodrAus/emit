#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    tracing_subscriber::fmt().init();

    let a = String::from("hello");

    emit::info!("Some text", value: #[emit::as_display] a);
}
