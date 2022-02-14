#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    tracing_subscriber::fmt().init();

    let a = String::from("hello");
    let c = 42;
    let err = std::io::Error::from(std::io::ErrorKind::Other);
    let f = {
        let mut map = std::collections::BTreeMap::new();
        map.insert("a", 42);
        map.insert("b", 17);
        map
    };

    emit::info!("Text and {a} and {b} and {#[emit::as_debug] c} or {d}",
        b: 17,
        #[emit::as_debug]
        d: String::from("short lived!"),
        err,
        #[emit::as_sval]
        f,
    );
}