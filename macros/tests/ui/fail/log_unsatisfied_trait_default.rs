#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

// Does not implement `Display`
struct Input;

fn main() {
    log!("Text and {a: Input} and more");
}
