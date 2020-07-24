#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    call("hello", 42);
}

fn call(string: &str, number: u64) {
    let d = number;
    let e = string;
    let f = 5;

    let kvs: &[(&str, antlog_macros_private::__private::Value)] = &[
        ("a", "a value".into()),
        #[debug]__log_private_capture!(b: 42),
        ("c", "c value".into()),
        #[debug]__log_private_capture!(d),
        #[display]__log_private_capture!(e),
        __log_private_capture!(f),
        #[debug]__log_private_capture!(g: e),
    ];

    println!("{:?}", kvs);
}
