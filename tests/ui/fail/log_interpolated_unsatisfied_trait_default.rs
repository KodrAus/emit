#![feature(stmt_expr_attributes, proc_macro_hygiene)]

// Does not implement `Display`
struct Input;

fn main() {
    emit::info!("Text \"and\" {a: Input} and more");
}
