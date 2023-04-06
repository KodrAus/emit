#![feature(stmt_expr_attributes, proc_macro_hygiene)]

extern crate emit;

use std::io;

fn main() {
    emit::to(|record| {
        // Just make sure there's a typed `std::error::Error` there
        let err = record.props().err().expect("missing error");

        println!("{}", err.downcast_ref::<io::Error>().expect("invalid error type"));
    });

    let err = io::Error::from(io::ErrorKind::Other);

    emit::info!("something went wrong ({err})");
}
