#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate emit;

// Does not implement `Display`
struct Input;

fn main() {
    info!("Text \"and\" {a: Input} and more");
}
