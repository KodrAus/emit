# `emit`

**Current Status: Proof-of-concept**

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

![The output of running the above program](https://github.com/KodrAus/emit/blob/main/asset/emit_term.png?raw=true)
