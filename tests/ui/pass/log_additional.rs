#![feature(stmt_expr_attributes, proc_macro_hygiene)]

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

    emit::info!("There's no replacements here",
        a,
        b: 17,
        #[emit::as_debug]
        c,
        d: String::from("short lived!"),
        err: e,
        #[emit::as_sval]
        f,
    );
}
