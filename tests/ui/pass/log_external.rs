#![feature(stmt_expr_attributes, proc_macro_hygiene)]

extern crate emit;

use uuid::Uuid;

fn main() {
    emit::target(|record| {
        // Just make sure we can fetch a `Uuid`
        let id = record.get("id").expect("missing id");
        let id = id.downcast_ref::<Uuid>().expect("not a uuid");

        println!("{}", id);
        println!("{}", sval_json::to_string(record).expect("failed to serialize"));
    });

    let id = Uuid::new_v4();

    emit::info!("something went wrong ({id})");
}
