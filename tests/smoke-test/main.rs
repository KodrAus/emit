#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    // Set up a global receiver for events
    emit::to(|evt| {
        println!("{}: {}", evt.lvl(), evt.msg());
    });

    // Emit an info event to the global receiver
    emit::info!("something went wrong at {id: 42}");

    // Format an event as a string
    let s = emit::format!("something went wrong at {id: 42}");
    println!("{}", s);

    // Use an event with an ad-hoc receiver
    emit::info!(
        to: |evt| {
            println!("{}", evt.msg());
        },
        "something went wrong at {id: 42}"
    );
}
