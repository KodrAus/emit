/*!
Structured diagnostics for Rust applications.

Emit is a structured logging framework for manually instrumenting Rust applications with an expressive syntax.

# Data model

## Events

All diagnostics in Emit are represented as _events_. An event is a notable change in the state of a system that is broadcast to outside observers. Events carry both a human-readable description of what triggered them as well as a structured payload that can be used to process them. Events are temporal; they may be anchored to a point in time at which they occurred, or may cover a span of time for which they are active.

The core event model includes:

- **Module (`module`):** The name of the component that triggered the event.
- **Extent (`ts_start`..`ts`):** The point or span of time for which the event is relevant.
- **Template (`tpl` and `msg`):** A human-readable description of the event that its properties may be interpolated into.
- **Properties (`props`):** The payload of the event.

## Extents

The extent of an event is the time for which the event is relevant. This may be a single point in time if the event was triggered by something happening, or a span of time if the event was started at one point and completed at a later one.

## Templates

The primary source of information in an event is its _template_. A template is a human-readable description of an event with holes to interpolate values into. Templates are responsible for both capturing local state to include in an event, and to format that state into a human-readable description.

Templates are a useful low-cardinality identifier for events.

Emit's templates are inspired by [Message Templates](https://messagetemplates.org).

## Properties

Emit's properties are structured key-value pairs where keys are strings and values are anything from primitive types to complex records and sequences. Values can use the data model of either `serde` or `sval`.

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

use emit_core::{extent::ToExtent, path::Path};

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
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    event::Event,
    extent::Extent,
    filter::Filter,
    level::Level,
    props::Props,
    rng::Rng,
    str::Str,
    template::Template,
    timer::Timer,
    timestamp::Timestamp,
    trace::{SpanId, TraceId},
    value::Value,
};

mod macro_hooks;
mod platform;

#[cfg(feature = "std")]
mod setup;
#[cfg(feature = "std")]
pub use setup::*;

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
