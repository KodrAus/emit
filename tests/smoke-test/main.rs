#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::target(|record| println!("{}", record.msg()));

    emit::info!("something went wrong ({id: 42})");
}
