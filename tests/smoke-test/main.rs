#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::target(|record| {
        println!("{}", record.message());
    });

    emit::info!("something went wrong at {id: 42}");
}
