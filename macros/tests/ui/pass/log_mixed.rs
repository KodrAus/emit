#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate antlog_macros;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let a = String::from("hello");
    let c = 42;
    let err = std::io::Error::from(std::io::ErrorKind::Other);
    let f = {
        let mut map = std::collections::BTreeMap::new();
        map.insert("a", 42);
        map.insert("b", 17);
        map
    };

    log!("Text and {a} and {b} and {#[debug] c} or {d}",
        b: 17,
        #[debug]
        d: String::from("short lived!"),
        err,
        #[sval]
        f,
    );
}
