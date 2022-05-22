#![feature(stmt_expr_attributes, proc_macro_hygiene)]
#![deny(warnings)]

fn main() {
    emit::info!("Text and {a: return 42}");
}
