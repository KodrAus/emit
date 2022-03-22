#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::target(|record| println!("{}", record.msg()));

    let id = 42u128;

    emit::info!("something went wrong ({id})");
}
