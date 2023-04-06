#![feature(stmt_expr_attributes, proc_macro_hygiene)]

extern crate emit;

fn main() {
    tracing_subscriber::fmt().init();

    emit::info!("something went wrong ({#[emit::as_display] err: 42})");
}
