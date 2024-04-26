/*!
Structured diagnostics for Rust applications.

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

# A guided tour of `emit`

To get started, add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"
```

## Configuring an emitter

`emit` needs to be configured with at least an [`Emitter`] that receives events, otherwise your diagnostics will go nowhere. At the start of your `main` function, use [`setup()`] to initialize `emit`. At the end of your `main` function, use [`setup::Init::blocking_flush`] to ensure all emitted events are fully flushed before returning.

Here's an example of a simple configuration with an emitter that prints events using [`std::fmt`]:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

In real applications, you'll want to use a more sophisticated emitter, such as:

- `emit_term`: Emit diagnostics to the console.
- `emit_file`: Emit diagnostics to a set of rolling files.
- `emit_otlp`: Emit diagnostics to a remote collector via OpenTelemetry Protocol.

For more advanced setup options, see the [`mod@setup`] module.

## Emitting events

`emit` uses macros with a special syntax to log events.

Here's an example of an event:

```
emit::emit!("Hello, World");
```

Using the `std::fmt` emitter from earlier, it will output:

```text
Event {
    module: "my_app",
    tpl: "Hello, world!",
    extent: Some(
        "2024-04-23T10:04:37.632304000Z",
    ),
    props: {},
}
```

This example is a perfect opportunity to introduce `emit`'s model of diagnostics: events. An event is a notable change in the state of a system that is broadcast to outside observers. Events are the combination of:

- `module`: The component that raised the event.
- `tpl`: A text template that can be rendered to describe the event.
- `extent`: The point in time when the event occurred, or the span of time for which it was active.
- `props`: A set of key-value pairs that define the event and capture the context surrounding it.

`emit`'s events are general enough to represent many observability paradigms including logs, distributed traces, metric samples, and more.

## Properties in templates

The string literal argument to the [`emit!`] macro is its template. Properties can be attached to events by interpolating them into the template between braces:

```
let user = "World";

emit::emit!("Hello, {user}!");
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-25T00:53:36.364794000Z",
    ),
    props: {
        "user": "World",
    },
}
```

`emit` uses Rust's field value syntax between braces in its templates, where the identifier becomes the key of the property. This is the same syntax used for struct field initialization. The above example could be written equivalently in other ways:

```
let greet = "World";

emit::emit!("Hello, {user: greet}!");
```

```
emit::emit!("Hello, {user: \"World\"}!");
```

In these examples we've been using the string `"World"` as the property value. Other primitive types such as booleans, integers, floats, and most library-defined datastructures like UUIDs and URIs can be captured by default in templates.

## Properties outside templates

Additional properties can be added to an event by listing them as field values after the template:

```
let user = "World";
let greeter = "emit";
let lang = "en";

emit::emit!("Hello, {user}!", lang, greeter);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-25T22:32:23.013651640Z",
    ),
    props: {
        "greeter": "emit",
        "lang": "en",
        "user": "World",
    },
}
```

Properties inside templates may be initialized outside of them:

```
emit::emit!("Hello, {user}", user: "World");
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`",
    extent: Some(
        "2024-04-25T22:34:25.536438193Z",
    ),
    props: {
        "user": "World",
    },
}
```

## Controlling event construction and emission

Control parameters appear before the template. They use the same field value syntax as properties, but aren't captured as properties on the event. They instead customize other aspects of the event.

The `module` control parameter sets the module:

```
emit::emit!(module: "my_module", "Hello, World!");
```

```text
Event {
    module: "my_module",
    tpl: "Hello, World!",
    extent: Some(
        "2024-04-25T22:42:52.180968127Z",
    ),
    props: {},
}
```

The `extent` control parameter sets the extent:

```
# use std::time::Duration;
emit::emit!(
    extent: emit::Timestamp::new(Duration::from_secs(1000000000)),
    "Hello, World!",
);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, World!",
    extent: Some(
        "2001-09-09T01:46:40.000000000Z",
    ),
    props: {},
}
```

The `when` control parameter only emits the event if it matches the given filter:

```
// This event matches the filter
emit::emit!(
    when: emit::filter::from_fn(|evt| evt.module() == "my_app"),
    "Hello, World!",
);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, World!",
    extent: Some(
        "2024-04-25T22:54:50.055493407Z",
    ),
    props: {},
}
```

```
// This event does not match the filter
emit::emit!(
    when: emit::filter::from_fn(|evt| evt.module() == "my_app"),
    module: "not_my_app",
    "Hello, World!",
);
```

```text

```

## Rendering templates

Templates are parsed at compile-time, but are rendered at runtime by passing the properties they capture back. The [`Event::msg`] method is a convenient way to render the template of an event using its properties. Taking an earlier example:

```
let user = "World";

emit::emit!("Hello, {user}!");
```

If we change our emitter to:

```
# let e =
emit::emitter::from_fn(|evt| println!("{}", evt.msg()))
# ;
```

then it will produce the output:

```text
Hello, World!
```
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
