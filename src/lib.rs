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

When significant operations are invoked in your application you can use _span events_ to time them while also corrolating any other events they emit into a trace hierarchy. This can be done using the [`span`], [`debug_span`], [`info_span`], [`warn_span`], and [`error_span`] macros. The macros use the same syntax as those for regular events:

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

# Observability signals

Emit doesn't hard-code common observability concepts into events. It instead relies on the presence well-known properties to carry that information.

### Logs

Events with a point extent can represent log records. Well-known properties related to logs include:

- **Level (`lvl`):** A traditional log level that describes the relative severity of an event for coarse-grained filtering.
    - **Debug:** A high-frequency point in the execution of an operation.
    - **Info:** A significant point in the execution of an operation.
    - **Warn:** An erroneous event that didn't cause its operation to fail.
    - **Error:** An erroneous event that caused its operation to fail.
- **Error (`err`):** An error that caused the event.

### Traces

Events with a span extent can represent spans in a distributed trace. Events in a distributed trace also need to carry a _trace id_ and _span id_. Well-known properties related to traces include:

- **Trace id (`trace_id`):** An identifier that marks an event as belonging to a distributed trace.
- **Span id (`span_id`):** An identifier that marks an event as belonging to a span of execution in a distributed trace.
- **Parent span id (`span_parent`):** An identifier that links the span id of an event to the span id of its parent.

Emit doesn't define any direct APIs for trace propagation or sampling. That responsibility is left up to the caller.

### Metrics

Emit's model for metrics is based on _aggregations_. A metric captures the result of applying an aggregation over an underlying timeseries data source within the extent to produce a sample. Events with a point extent can represent cumulative metric samples. Events with a span extent can represent delta metric samples. Well-known properties related to metrics include:

- **Metric name (`metric_name`):** The name of a data source that marks an event as representing a metric sampled from that source.
- **Metric aggregation (`metric_agg`):** The aggregation over the data source the metric sample was computed with.
    - **Last:** The latest value in the underlying source.
    - **Sum:** The sum of all values in the underlying source.
    - **Count:** The count of all values in the underlying source. A count is a monotonic sum of ones.
- **Metric value (`metric_value`):** The sampled value from the metric source.
- **Metric unit (`metric_unit`):** The unit the sampled value is in.

Emit's metric support can represent common cases of counters and gauges, but can't express the full fidelity of other models.
*/

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::extent::ToExtent;

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, emitter, empty, event, extent, filter, path, props, rng, runtime, str, template,
    timestamp, value, well_known,
};

pub mod frame;
pub mod level;
pub mod metric;
pub mod timer;
pub mod trace;

pub use self::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, event::Event, extent::Extent, filter::Filter,
    frame::Frame, level::Level, path::Path, props::Props, rng::Rng, str::Str, template::Template,
    timer::Timer, timestamp::Timestamp, value::Value,
};

mod macro_hooks;
mod platform;

#[cfg(feature = "std")]
pub mod setup;
#[cfg(feature = "std")]
pub use setup::{setup, Setup};

#[track_caller]
fn base_emit(
    to: impl Emitter,
    source: Path,
    when: impl Filter,
    ctxt: impl Ctxt,
    ts: impl ToExtent,
    tpl: Template,
    props: impl Props,
) {
    ctxt.with_current(|ctxt| {
        let evt = Event::new(source, ts, tpl, props.chain(ctxt));

        if when.matches(&evt) {
            to.emit(&evt);
        }
    });
}

#[doc(hidden)]
pub mod __private {
    pub use crate::macro_hooks::*;
    pub use core;
}
