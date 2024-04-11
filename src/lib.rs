/*!
Structured diagnostics for Rust applications.

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org). All diagnostics in `emit` are represented as [`Event`]s. An event is a notable change in the state of a system that is broadcast to outside observers. Events carry both a human-readable description of what triggered them in the form of a [`Template`]. They also have a structured payload of [`Props`] that can be used to process them. Events have a temporal [`Extent`]; they may be anchored to a point in time at which they occurred, or may cover a span of time for which they are active. Together, this information provides a solid foundation for building tailored diagnostics into your applications.

# Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"

[dependencies.emit_term]
version = "*"
```

`emit` needs to be configured with at least an [`Emitter`] that sends events somewhere. In this example we're using `emit_term` to write events to the console. Other emitters exist for rolling files and OpenTelemetry's wire format.

At the start of your `main` function, use [`setup()`] to initialize `emit`. At the end of your `main` function, use [`setup::Init::blocking_flush`] to ensure all emitted events are fully flushed to the outside target.

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

# Logging events

When something significant happens in your application you can emit an event for it using the [`emit`], [`debug`], [`info`], [`warn`], and [`error`] macros. The macros accept a string literal template that may have properties captured and interpolated using Rust's field value syntax between braces:

```
let user = "Rust";
let id = 42;

emit::info!("Hello, {user}", id);
```

In this example, the template is `"Hello, {user}"`, and the properties are `{"user": "Rust", "id": 42}`.

Properties are field values, so identifiers may be given values inline either in the template itself or after it:

```
emit::info!("Hello, {user: \"Rust\"}", id: 42);
```

Properties can also be mentioned by name in the template, but initialized outside of it:

```
emit::info!("Hello, {user}", user: "Rust", id: 42);
```

# Tracing functions

When significant operations are invoked in your application you can use _span events_ to time them, also linking any other events they emit into a correlated hierarchy. This can be done using the [`span`], [`debug_span`], [`info_span`], [`warn_span`], and [`error_span`] macros. The macros use the same syntax as those for regular events:

```
#[emit::info_span!("Invoke with {user}")]
fn my_function(user: &str) {
    // Function body..
}
```

When `my_function` completes, an event will be emitted with the time it took to execute.

## Completion

The span macros accept an argument called `arg` _before_ the template for an identifier that can be used to manually complete the span. This can be useful to complete the span differently based on control-flow:

```
# type Error = Box<dyn std::error::Error>;
#[emit::info_span!(arg: span, "Parse {id}")]
fn my_function(id: &str) -> Result<i32, Error> {
    match id.parse() {
        Ok(id) => Ok(id),
        Err(err) => {
            span.complete(|extent| emit::error!(extent, "Parse {id} failed", err));

            Err(err.into())
        }
    }
}
```

In this example, we use the `arg` field value before the template to assign a local variable `span` that represents the span for our function. The type of `span` is a [`timer::TimerGuard`]. In the `Ok` branch, we let the span complete normally. In the `Err` branch, we complete the span manually with the error produced.

# Aggregating metrics

You can periodically aggregate and emit events for metrics tracked by your applications. This can be done using the [`emit`] macro along with some well-known properties. An event is considered a metric if it carries:

- `metric_name`: The source of the metric.
- `metric_agg`: The aggregation applied to the source to produce its value.
- `metric_value`: The value of the metric.

In SQL terms, you can think of a metric as:

```sql
select metric_agg(x) as metric_value from metric_name
```

`emit` doesn't have any infrastructure for collecting and storing metrics themselves; this is left up to the application.

## Cumulative metrics

Events with a point extent are considered cumulative metrics. The value is the result of applying the aggregation over the entire lifetime of the metric up to that timestamp.

```
emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    metric_agg: "count",
    metric_name: "requests_received",
    metric_value: 17,
);
```

In this example, the total number of requests received over the lifetime of the application is `17`.

## Delta metrics

Events with a span extent are considered delta metrics. The value is the result of applying the aggregation over that time range.

```
let now = emit::runtime::shared().now();

emit::emit!(
    extent: now..now + Duration::from_secs(60),
    "{metric_agg} of {metric_name} is {metric_value}",
    metric_agg: "count",
    metric_name: "requests_received",
    metric_value: 3,
);
```

In this example, there have been `3` new requests received over the last minute.

## Histogram metrics

Events with a span extent where the metric value is also a sequence are considered histograms. Each element in the sequence is a bucket. The width of each bucket is implied as `extent.len() / metric_value.len()`.

```
let now = emit::runtime::shared().now();

emit::emit!(
    extent: now..now + Duration::from_secs(60),
    "{metric_agg} of {metric_name} is {metric_value}",
    metric_agg: "count",
    metric_name: "requests_received",
    #[emit::as_value]
    metric_value: &[
        3,
        6,
        0,
        8,
    ],
);
```

In this example, the requests received are collected into 15 second buckets (`60s / 4`).

# Property capturing

By default, properties are required to implement `Display + 'static`. This can be customized using the [`as_debug`], [`as_display`], [`as_sval`], [`as_serde`], and [`as_error`] attribute macros. These attributes can be applied to properties either inside or outside of the template:

```
#[derive(Serlialize)]
struct Work {
    pub id: usize,
    pub content: String,
}

let work = Work {
    id: 42,
    content: String::from("apply latest migrations"),
};

emit::info!("Working on {#[emit::as_serde] work}");
```

In this example, the property `work` is captured as a fully structured object using its `serde::Serialize` implementation.

## Complex key names

Property key names are derived from their identifiers by default, but can be customized using the [`key`] attribute macro:

```
let user = "Rust";

emit::info!("Hello, {user}", #[emit::key("user.name")] user);
```

In the above example, the identifier `user` just links the hole in the template with the property supplied afterwards. The name of the property in the template and on the event becomes `"user.name"`.

# Control parameters

Field values supplied in templates and after it are properties that are captured in the event. Field values can also appear _before_ the template too. These field values are _control parameters_ that customize how events are constructed and emitted. For example, the macros accept a `to` field value to supply an alternative emitter for an event:

```
let user = "Rust";

emit::info!(to: emit::emitter::from_fn(|evt| println!("{}", evt)), "Hello", user);
```

Think of field values before the template like optional function arguments.

# Extents and timestamps

The extent of an event is the time for which it is relevant. This may be a single point in time if the event was triggered by something happening, or a span of time if the event was started at one point and completed at a later one. Extents are represented by the [`Extent`] type, and timestamps by the [`Timestamp`] type.

Events are automatically assigned an extent with the current timestamp when they're emitted based on a [`Clock`]. This can be customized with the `extent` control parameter on the macros.

This is an example of a point event:

```
use std::time::Duration;

let now = emit::Timestamp::new(Duration::from_secs(1711317957));

emit::info!(extent: now, "A point event");
```

This is an example of a span event:

```
# use std::time::Duration;
# let now = emit::Timestamp::new(Duration::from_secs(1711317957));
let later = now + Duration::from_secs(1000);

emit::info!(extent: now..later, "A span event");
```

The [`Timer`] type provides a convenient way to produce an extent that covers a time range.

## Getting the current timestamp

You can read the current timestamp from a clock using [`Clock::now`]:

```
let now = emit::runtime::shared().now();
```

# Ambient context

`emit` doesn't require threading loggers through your program directly. You can store ambient state you want events to carry in the ambient [`Ctxt`]. `emit`'s context is a stack that can be managed either directly for synchronous operations, or through a future for asynchronous ones.

The [`Frame`] type is a wrapper over a [`Ctxt`] that handles pushing and popping a set of properties automatically.

Any properties captured by the span macros will be pushed onto the current context.

## Setting ambient context

The most straightfoward way to push ambient context is using the span macros:

```
#[emit::span("Greeting", span_id: "86d871f54f5b4e23")]
{
    emit::info!("Hello, {user}", user: "Rust", id: 42);
}
```

You can also push properties onto the ambient context directly with [`Frame::push`]:

```
let frame = emit::Frame::push(emit::runtime::shared(), emit::props! {
    span_id: "86d871f54f5b4e23"
});

let _guard = frame.enter();

emit::info!("Hello, {user}", user: "Rust", id: 42);
```

## Reading ambient context

You can read the current properties in the ambient context with [`Frame::with`]:

```
emit::Frame::current().with(|props| {
    let span_id = props.get("span_id");

    // ..
});
```

This example also uses [`Frame::current`] instead of [`Frame::push`] to get a frame of the current ambient context so its properties can be read.

## Propagating ambient context across threads

Ambient context is not guaranteed to propagate across threads, so when spawning background work that should retain the context of its creator, you'll need to grab the current context to send with the work:

```
thread::spawn({
    let frame = emit::Frame::current(emit::runtime::shared());

    move || frame.call(move || {
        // ..
    })
});
```

In async code, you can typically wrap the future representing the background work in the current context:

```
tokio::spawn(
    emit::Frame::current(emit::runtime::shared()).in_future(async {
        // ..
    })
);
```

# Runtimes

All functionality needed to emit events is encapsulated in a [`runtime::Runtime`]. This type carries the following key components:

- [`Filter`]: Determines whether an event should be emitted.
- [`Emitter`]: The target of emitted events.
- [`Ctxt`]: Storage for ambient context.
- [`Clock`]: Reads the wall-clock time.
- [`Rng`]: Source of randomness.

When calling the macros, the runtime is implicitly assumed to be [`runtime::shared`], which is what [`Setup::init`] will configure. You can override the runtime with the `rt` control parameter in the macros.

The default context will use thread-local storage to reduce synchronization, so is only available when running on a host that supports it. The clock and randomness also rely on the host platform, so are unavailable in embedded environments unless a bespoke implementation is configured.
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
    clock::Clock, ctxt::Ctxt, emitter::Emitter, event::Event, extent::Extent, filter::Filter,
    frame::Frame, level::Level, metric::Metric, path::Path, props::Props, rng::Rng, span::Span,
    str::Str, template::Template, timer::Timer, timestamp::Timestamp, value::Value,
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
