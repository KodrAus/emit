#![feature(stmt_expr_attributes, proc_macro_hygiene)]

extern crate emit;

use std::io;

fn main() {
    emit::target(|record| {
        // Just make sure there's a typed `std::error::Error` there
        assert!(record.source().is_some());

        println!("{}", sval_json::to_string(record).expect("failed to serialize"));
    });

    let err = io::Error::from(io::ErrorKind::Other);

    emit::info!("something went wrong ({#[emit::source] source: err})");
}
