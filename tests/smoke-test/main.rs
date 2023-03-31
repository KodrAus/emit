#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::to(|evt| {
        println!("{}: {}", evt.level(), evt.message());
    });

    emit::info!("something went wrong at {id: 42}");
}
