#![feature(stmt_expr_attributes, proc_macro_hygiene)]

extern crate emit;

use std::io;

fn main() {
    emit::set(|record| {
        println!("{}", record.msg());

        assert!(record.source().is_some());
    });

    let err = io::Error::from(io::ErrorKind::Other);

    emit::emit!("something went wrong ({source: err})");
}
