#![feature(stmt_expr_attributes, proc_macro_hygiene)]
#![deny(unused_variables)]

fn main() {
    let a = String::from("hello");

    // Unused by the log statement
    let c = 42;

    emit::info!("A log with cfgs {#[cfg(disabled)] b: 17}",
        a,
        #[emit::as_debug]
        #[cfg(disabled)]
        c,
        d: String::from("short lived!"),
    );
}
