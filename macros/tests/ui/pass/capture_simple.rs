#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    call("hello", 42);
}

fn call(string: &str, number: u64) {
    let d = number;
    let e = string;

    let _kvs: &[(&str, antlog_macros_private::__private::Value)] = &[
        ("a", "a value".into()),
        #[debug(key = "b")] 42,
        ("c", "c value".into()),
        #[debug(key = "d")] d,
        #[display] e,
    ];
}
