#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[macro_use]
extern crate emit_macros;

#[macro_use]
extern crate serde_derive;

use uuid::Uuid;

#[derive(Serialize)]
struct Work {
    id: Uuid,
    description: String,
    size: usize,
}

impl Work {
    pub fn complete(self) {}
}

fn main() {
    tracing_subscriber::fmt().init();

    let work = Work {
        id: Uuid::new_v4(),
        description: String::from("upload all the documents"),
        size: 1024,
    };

    emit!("scheduling background work {description: work.description} ({id: work.id})", #[serde] work);

    work.complete();
}
