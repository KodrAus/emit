#![feature(stmt_expr_attributes, proc_macro_hygiene)]
#![allow(unused_variables)]

#[macro_use]
extern crate emit;

fn main() {
    tracing_subscriber::fmt().init();

    let a = String::from("hello");
    let c = 42;

    info!("A log with cfgs {#[cfg(disabled)] b: 17}",
        a,
        #[as_debug]
        #[cfg(disabled)]
        c,
        d: String::from("short lived!"),
    );
}
