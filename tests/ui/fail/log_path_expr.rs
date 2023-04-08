#![feature(stmt_expr_attributes, proc_macro_hygiene)]

struct Data {
    a: i32,
}

fn main() {
    let data = Data {
        a: 42,
    };

    emit::info!("Logging with a deeply nested {data.a}");
}
