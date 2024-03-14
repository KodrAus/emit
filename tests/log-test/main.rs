#![feature(stmt_expr_attributes, proc_macro_hygiene)]

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::max())
        .init();

    let _ = emit::setup().to(emit_log::global_logger()).init();

    #[emit::span("My span", a: 42)]
    {
        emit::info!("Hello, world!");
    }
}
