#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate emit;

fn main() {
    tracing_subscriber::fmt().init();

    fn call(v: &&&str) {
        info!("Logging with a deeply nested {value: *v}");
    }
}
