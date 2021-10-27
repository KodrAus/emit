#![feature(stmt_expr_attributes, proc_macro_hygiene)]
#![allow(unused_variables)]

fn main() {
    tracing_subscriber::fmt().init();

    let a = String::from("hello");
    let c = 42;

    emit::info!("A log with cfgs {#[cfg(disabled)] b: 17}",
        a,
        #[emit::as_debug]
        #[cfg(disabled)]
        c,
        d: String::from("short lived!"),
    );
}
