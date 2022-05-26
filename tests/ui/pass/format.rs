#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    let msg = emit::format!("A string {template: 42}");

    assert_eq!("A string 42", msg);
}
