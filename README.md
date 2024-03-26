# `emit`

**Current Status: Proof-of-concept**

This library is a playground for modern structured logging techniques for Rust, based on the work of `log` and `tracing`.

It's just a proof-of-concept that will need a lot more work to be polished into a consumable artifact, but sketches out a lot of design space.

For some idea of what it can do, see the `tests/smoke-test` example.

```rust
use std::time::Duration;

fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    greet("Rust");

    rt.blocking_flush(Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```
