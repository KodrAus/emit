#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    let world = "world";
    let structurued = "structured";
    let a = 42;
    let c = 17;

    log!("Hello {world}! This message is {#[display] structured}", { a, b: c });
}
