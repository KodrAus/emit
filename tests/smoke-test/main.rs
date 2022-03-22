#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    emit::target(|record| {
        #[cfg(feature = "json")]
        {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            let _ = sval_json::to_writer(&mut stdout, &record);
        }
        #[cfg(not(feature = "json"))]
        {
            println!("{}", record.msg());
        }
    });

    emit::info!("something went wrong at {id: 42}");
}
