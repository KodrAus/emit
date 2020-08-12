#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    let a = String::from("hello");
    let c = 42;
    let err = std::io::Error::from(std::io::ErrorKind::Other);

    log!("Text and {a} and {b} and {#[debug] c} or {d}",
        b: 17,
        #[debug]
        d: String::from("short lived!"),
        err,
    );
}
