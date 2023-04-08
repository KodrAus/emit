#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    let a = String::from("hello");

    emit::info!("Some text", value: #[emit::as_display] a);
}
