#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    let a = String::from("hello");
    let c = 42;

    let kvs = log!("Text and {a} and {b: 17} and {#[debug] c}");

    println!("{:?}", kvs);
}
