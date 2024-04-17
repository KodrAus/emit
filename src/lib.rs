/*!
Structured diagnostics for Rust applications.

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

All diagnostics in `emit` are represented as [`Event`]s. An event is a notable change in the state of a system that is broadcast to outside observers. Events carry both a human-readable description of what triggered them in the form of a [`Template`] and a structured payload of [`Props`] that can be used to process them. Events have a temporal [`Extent`]; they may be anchored to a point in time at which they occurred, or may cover a span of time for which they are active. Together, this information provides a solid foundation for building tailored diagnostics into your applications.

# Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"

[dependencies.emit_term]
version = "*"
```

`emit` needs to be configured with at least an [`Emitter`] that receives events, otherwise your diagnostics will go nowhere. In this example we're using `emit_term` to write events to the console. Other emitters exist for rolling files and OpenTelemetry's wire format.

At the start of your `main` function, use [`setup()`] to initialize `emit`. At the end of your `main` function, use [`setup::Init::blocking_flush`] to ensure all emitted events are fully flushed before returning.

```
# mod emit_term { pub fn stdout() -> impl emit::Emitter + Send + Sync + 'static { emit::emitter::from_fn(|_| {}) } }
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

For more advanced setup options, see the [`setup`] module.

# Logging events

`emit` uses macros with a special syntax to log events. Here's an example of an event:

```
emit::emit!("Hello, World");
```

```text
Event {
    module: "emit_test",
    extent: Some(
        Extent {
            range: 2024-04-15T20:58:29.222187000Z..2024-04-15T20:58:29.222187000Z,
            is_span: false,
        },
    ),
    msg: "Hello, world!",
    tpl: "Hello, world!",
    props: {},
}
```

In simple cases, `emit` looks a lot like the built-in `println!`.
*/

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::extent::ToExtent;

pub use std::module_path as module;

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, emitter, empty, event, extent, filter, path, props, rng, runtime, str, template,
    timestamp, value, well_known,
};

pub mod frame;
pub mod kind;
pub mod level;
pub mod metric;
pub mod span;
pub mod timer;

pub use self::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::Event, extent::Extent,
    filter::Filter, frame::Frame, kind::Kind, level::Level, metric::Metric, path::Path,
    props::Props, rng::Rng, span::Span, str::Str, template::Template, timer::Timer,
    timestamp::Timestamp, value::Value,
};

mod macro_hooks;
mod platform;

#[cfg(feature = "std")]
pub mod setup;
#[cfg(feature = "std")]
pub use setup::{setup, Setup};

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
