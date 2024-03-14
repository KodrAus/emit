/*!
Structured diagnostics for Rust applications.

Emit is a structured logging framework for manually instrumenting Rust applications with an expressive syntax.

# Events

All diagnostics in Emit are represented as _events_. An event is a notable change in the state of a system that is made available to outside observers. Events carry both a human-readable description of what triggered them as well as a structured payload that can be used to process them. Events are temporal; they may be anchored to a point in time at which they occurred, or may cover a span of time for which they are active.

## Core data model

The core event model includes:

- **Module (`module`):** The name of the component that triggered the event.
- **Extent (`ts_start`..`ts`):** The point or span of time for which the event is relevant.
- **Template (`tpl`):** A human-readable description of the event that its properties may be interpolated into.
- **Properties (`props`):** The payload of the event.

## Extensions

Emit doesn't hard-code common observability concepts into events. It instead relies on the presence well-known properties to carry that information.

### Logging

- **Level (`lvl`):** A traditional log level that describes the relative severity of an event for coarse-grained filtering.
- **Error (`err`):** An error that caused the event.

### Tracing

- **Trace id (`trace_id`):** An identifier that marks an event as belonging to a distributed trace.
- **Span id (`span_id`):** An identifier that marks an event as belonging to a span of execution in a distributed trace.
- **Parent span id (`span_parent`):** An identifier that links the span id of an event to the span id of its parent.

### Metrics

- **Metric name (`metric_name`):** The name of a data source that marks an event as representing a metric sampled from that source.
- **Metric aggregation (`metric_agg`):** The aggregation over the data source the metric sample was computed with.
- **Metric value (`metric_value`):** The sampled value from the metric source.
- **Metric unit (`metric_unit`):** The unit the sampled value is in.
*/

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use emit_core::{extent::ToExtent, path::Path};

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::{
    clock, ctxt, emitter, empty, event, extent, filter, props, rng, runtime, str, template,
    timestamp, value, well_known,
};

pub mod frame;
pub mod id;
pub mod level;
pub mod metrics;
pub mod timer;

pub use self::{
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    event::Event,
    extent::Extent,
    filter::{always, Filter},
    frame::FrameCtxt,
    id::{IdRng, SpanId, TraceId},
    level::Level,
    props::Props,
    rng::Rng,
    str::Str,
    template::Template,
    timer::Timer,
    timestamp::Timestamp,
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
