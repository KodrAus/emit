#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    let a = String::from("hello");
    let c = 42;

    log!("There's no replacements here", {
        a,
        b: 17,
        #[debug]
        c,
        d: String::from("short lived!"),
    });
}
