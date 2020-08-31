#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate emit_macros;

fn main() {
    tracing_subscriber::fmt().init();

    let a = String::from("hello");
    let c = 42;
    let e = std::io::Error::from(std::io::ErrorKind::Other);
    let f = {
        let mut map = std::collections::BTreeMap::new();
        map.insert("a", 42);
        map.insert("b", 17);
        map
    };

    emit!("Text and {a} and {b: 17} and {#[debug] c} or {#[display] d: String::from(\"short lived!\")} and {err: e} and {#[sval] f}");
}
