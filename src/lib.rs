/*!
Structured diagnostics for Rust applications.

Emit is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org). All diagnostics in Emit are represented as _events_. An event is a notable change in the state of a system that is broadcast to outside observers. Events carry both a human-readable description of what triggered them as well as a structured payload that can be used to process them. Events are temporal; they may be anchored to a point in time at which they occurred, or may cover a span of time for which they are active.

# Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"

[dependencies.emit_term]
version = "*"
```

`emit` needs to be configured with an _emitter_ that sends events somewhere. In this example we're using `emit_term` to write events to the console. Other emitters exist for rolling files and OpenTelemetry's wire format.

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

When something significant happens in your application you can emit an _event_ for it. This can be done using the [`emit`], [`debug`], [`info`], [`warn`], and [`error`] macros. The macros accept a string literal _template_ that may have _properties_ captured and interpolated into it using Rust's field value syntax:

```
let user = "Rust";

emit::info!("Hello, {user}");
```

This macro expands roughly to:

```
let user = "Rust";

// emit::info!("Hello, {user}");
let rt = emit::runtime::shared();

let extent = rt.now();
let props = &[
    ("user", user),
];

rt.emit(emit::Event::new(
    "some_app::some_module",
    extent,
    emit::Template::new(&[
        emit::template::Part::text("Hello, "),
        emit::template::Part::hole("user"),
    ]),
    props,
));
```

Properties can also be captured after the template:

```
let user = "Rust";

emit::info!("Hello", user);
```

Properties may be named or initialized directly in the template:

```
emit::info!("Hello, {user: \"Rust\"}");
```

Properties can also be named or initialized after the template:

```
emit::info!("Hello", user: "Rust");
```

Field values can also appear _before_ the template to customize how events are constructed and emitted. Field values before the template follow a fixed schema. For example, the macros accept a `module` field value to set the name of the containing module for an event:

```
let user = "Rust";

emit::info!(module: "my_mod", "Hello", user);
```

Think of field values before the template like optional function arguments, and field values after the template like a spread of extra parameters.

# Tracing functions

When significant operations are invoked in your application you can use _span events_ to time them while also corrolating any other events they emit into a trace hierarchy. This can be done using the [`span`], [`debug_span`], [`info_span`], [`warn_span`], and [`error_span`] macros. The macros use the same syntax as those for regular events:

```
#[emit::info_span!("Invoke with {user}")]
fn my_function(user: &str) {
    // Function body..
}
```

When `my_function` completes, an event will be emitted with the time it took to execute. This macro expands roughly to:

```
// #[emit::info_span!("Invoke with {user}")]
fn my_function(user: &str) {
    let rt = emit::runtime::shared();

    let timer = emit::Timer::start(rt).on_drop(|extent| {
        let props = &[
            ("user", user),
        ];

        rt.emit(emit::Event::new(
            "some_app::some_module",
            extent,
            emit::Template::new(&[
                emit::template::Part::text("Invoke with "),
                emit::template::Part::hole("user"),
            ]),
            props,
        ));
    });

    // Function body..

    drop(timer);
}
```

## Completion

The span macros accept an argument called `arg` _before_ the template for an identifier to give the [`timer::TimerGuard`] that can be used to manually complete the span. This can be useful to complete the span differently based on control-flow:

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

In this example, we use the `arg` field value before the template to assign a local variable `span` that represents the span for our function. In the `Ok` branch, we let the span complete normally. In the `Err` branch, we complete the span manually with the error produced.

# Extents and timestamps

The extent of an event is the time for which it is relevant. This may be a single point in time if the event was triggered by something happening, or a span of time if the event was started at one point and completed at a later one. Extents are represented by the [`Extent`] type, and timestamps by the [`Timestamp`] type.

Events are automatically assigned an extent with the current timestamp when they're emitted based on a [`Clock`]. This can be customized with the `extent` field value on the macros.

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

Span events can wrap a clock in a [`Timer`] to produce an extent that covers a time range.

## Observability signals

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

# Context

Emit doesn't require threading loggers through your program directly. You can store ambient state you want events to carry in the current _context_. Emit's context is a stack that can be managed either directly for synchronous operations, or through a future for asynchronous ones.

# Runtime

The set of components needed to produce, receive, filter, and emit events is encapsulated in a _runtime_. A system will typically configure the built-in shared runtime and use it, but any or multiple runtimes can be used independantly.
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
