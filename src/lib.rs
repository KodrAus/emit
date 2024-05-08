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

Producing useful diagnostics in your applications is a critical aspect of building robust and maintainable software. If you don't find your diagnostics useful in development, then you won't find them useful in production either when you really need them. `emit` is a tool for application developers, designed to be straightforward to configure and integrate into your projects with low conceptual overhead.

`emit` uses macros with a special syntax to log events. Here's an example of an event:

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

### Properties in templates

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

### Properties outside templates

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

### Controlling event construction and emission

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

The `props` control parameter adds a base set of [`Props`] to an event in addition to any added through the template:

```
emit::emit!(
    props: emit::props! {
        lang: "en",
    },
    "Hello, {user}!",
    user: "World",
);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-29T03:36:07.751177000Z",
    ),
    props: {
        "lang": "en",
        "user": "World",
    },
}
```

### Controlling property capturing

Property capturing is controlled by regular Rust attributes. For example, the [`key`] attribute can be used to set the key of a property to an arbitrary string:

```
emit::emit!(
    "Hello, {user}!",
    #[emit::key("user.name")]
    user: "World",
);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user.name`!",
    extent: Some(
        "2024-04-26T06:33:59.159727258Z",
    ),
    props: {
        "user.name": "World",
    },
}
```

By default, properties are captured based on their [`std::fmt::Display`] implementation. This can be changed by applying one of the `as` attributes to the property. For example, applying the [`as_debug`] attribute will capture the property using uts [`std::fmt::Debug`] implementation instead:

```
#[derive(Debug)]
struct User {
    name: &'static str,
}

emit::emit!(
    "Hello, {user}!",
    #[emit::as_debug]
    user: User {
        name: "World",
    },
);
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-26T06:29:55.778795150Z",
    ),
    props: {
        "user": User {
            name: "World",
        },
    },
}
```

In the above example, the structure of the `user` property is lost. It can be formatted using the `std::fmt` machinery, but if serialized to JSON it would produce a string:

```json
{
    "module": "my_app",
    "tpl": "Hello, `user`!",
    "extent": "2024-04-26T06:29:55.778795150Z",
    "props": {
        "user": "User { name: \"World\" }"
    }
}
```

To retain the structure of complex values, you can use the [`as_serde`] or [`as_sval`] attributes:

```
# #[cfg(not(feature = "serde"))] fn main() {}
# #[cfg(feature = "serde")]
# fn main() {
#[derive(serde::Serialize)]
struct User {
    name: &'static str,
}

emit::emit!(
    "Hello, {user}!",
    #[emit::as_serde]
    user: User {
        name: "World",
    },
);
# }
```

Represented as JSON, this event will instead produce:

```json
{
    "module": "my_app",
    "tpl": "Hello, `user`!",
    "extent": "2024-04-26T06:29:55.778795150Z",
    "props": {
        "user": {
            "name": "World"
        }
    }
}
```

Primitive types like booleans and numbers are always structure-preserving, so `as` attributes don't need to be applied to them.

## Filtering events

Managing the volume of diagnostic data is an important activity in application development to keep costs down and make debugging more efficient. Ideally, applications would only produce useful diagnostics, but reality demands tooling to limit volume at a high-level. `emit` lets you configure a [`Filter`] during [`setup()`] to reduce the volume of diagnostic data. A useful filter is [`level::min_by_path_filter`], which only emits events when they are produced for at least a given [`Level`] within a given module:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .emit_when(emit::level::min_by_path_filter([
            ("my_app::submodule", emit::Level::Info)
        ]))
        .init();

    emit::debug!("Up and running");

    submodule::greet("World");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}

mod submodule {
    pub fn greet(user: &str) {
        emit::debug!("Preparing to greet {user}");
        emit::info!("Hello, {user}!");
    }
}
```

```text
Event {
    module: "my_app",
    tpl: "Up and running",
    extent: Some(
        "2024-04-29T04:31:24.085826100Z",
    ),
    props: {
        "lvl": debug,
    },
}
Event {
    module: "my_app::submodule",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-29T04:31:24.086327500Z",
    ),
    props: {
        "lvl": info,
        "user": "World",
    },
}
```

Filters can apply to any feature of a candidate event. For example, this filter only matches events with a property `lang` that matches `"en"`:

```
# let e =
emit::filter::from_fn(|evt| {
    use emit::Props as _;

    evt.props().pull::<emit::Str, _>("lang") == Some(emit::Str::new("en"))
});
# ;
```

The `when` control parameter of the emit macros can be used to override the globally configured filter for a specific event:

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

This can be useful to guarantee an event will always be emitted, regardless of any filter configuration:

```
// This event is never filtered out
emit::emit!(
    when: emit::filter::always(),
    "Hello, World!",
);
```

## Tracing operations

When your application executes key operations, you can emit span events that dover the time they were active. Any other operations involved in that execution, or any other events emitted during it, will be correlated through identifiers to form a hierarchical call tree. Together, these events form a trace, which in distributed systems can involve operations executed by other services. Traces are a useful way to build a picture of service dependencies in distributed applications, and to identify performance problems across them.

`emit` supports tracing operations through attribute macros on functions. These macros use the same syntax as those for emitting regular events:

```
# use std::{thread, time::Duration};
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms))
}

wait_a_bit(1200);
```

```text
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-27T22:40:24.112859000Z".."2024-04-27T22:40:25.318273000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "span_id": 71ea734fcbb4dc41,
        "trace_id": 6d6bb9c23a5f76e7185fb3957c2f5527,
        "sleep_ms": 1200,
    },
}
```

When the annotated function returns, a span event for its execution is emitted. The extent of a span event is a range, where the start is the time the function began executing, and the end is the time the function returned.

On nightly compilers, the same attributes can also be applied to blocks instead of functions:

```
#![feature(proc_macro_hygiene, stmt_expr_attributes)]

# use std::{thread, time::Duration};
# fn main() {
let sleep_ms = 1200;

#[emit::span("wait a bit", sleep_ms)]
{
    thread::sleep(Duration::from_millis(sleep_ms))
}
# }
```

Asynchronous functions are also supported:

```
# use std::{thread, time::Duration};
# fn main() {}
# async fn sleep(_: Duration) {}
# async fn main_async() {
#[emit::span("wait a bit", sleep_ms)]
async fn wait_a_bit(sleep_ms: u64) {
    sleep(Duration::from_millis(sleep_ms)).await
}

wait_a_bit(1200).await;
# }
```

Span events may also be created manually:

```
# use std::{time::Duration, thread};
use emit::Filter;

let sleep_ms = 1200;

let rt = emit::runtime::shared();

// Create a span
let mut span = emit::Span::filtered_new(
    |span| rt.filter().matches(&span),
    "my_app",
    emit::Timer::start(rt.clock()),
    "wait a bit",
    emit::span::SpanCtxt::current(rt.ctxt()).new_child(rt.rng()),
    emit::Empty,
    |span| {
        emit::emit!(
            event: span,
            when: emit::filter::always(),
            "wait a bit",
        );
    },
);

// Push the span onto the current context
let frame = span.push_ctxt(
    rt.ctxt(),
    emit::props! {
        sleep_ms,
    },
);

// Execute some operation within the frame
frame.call(move || {
    // Your code goes here
    thread::sleep(Duration::from_millis(sleep_ms));

    // Make sure you complete the span in the frame.
    // This is especially important for futures, otherwise the span may
    // complete before the future does
    span.complete();
});
```

Spans can also be emitted directly as regular events:

```
# use std::{thread, time::Duration, panic};
use emit::{well_known::EVENT_KIND_SPAN, Filter};

let sleep_ms = 1200;

let rt = emit::runtime::shared();

let timer = emit::Timer::start(rt.clock());
let props = emit::span::SpanCtxt::current(rt.ctxt()).new_child(rt.rng());

// Check whether the span should be created or not
let frame = if rt.filter().matches(&emit::event! {
    extent: timer.start_timestamp(),
    props,
    "wait a bit",
    event_kind: EVENT_KIND_SPAN,
}) {
    emit::Frame::push(Some(rt.ctxt()), props)
} else {
    emit::Frame::current(None)
};

// Execute some operation within the frame
frame.call(|| {
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        // Your code goes here
        thread::sleep(Duration::from_millis(sleep_ms));
    }));

    // Emit the span event at the end of the scope
    // We do this regardless of panics
    emit::emit!(
        extent: timer,
        "wait a bit",
        event_kind: EVENT_KIND_SPAN,
        sleep_ms,
    );

    match r {
        Ok(r) => r,
        Err(r) => panic::resume_unwind(r),
    }
});
```

The data model of spans is an extension of `emit`'s events. Span events include the following well-known properties:

- `event_kind`: with a value of `"span"` to indicate that the event is a span.
- `span_name`: a name for the operation the span represents. This defaults to the template.
- `span_id`: an identifier for this specific invocation of the operation.
- `parent_id`: the `span_id` of the operation that invoked this one.
- `trace_id`: an identifier shared by all events in a distributed trace. A `trace_id` is assigned by the first operation.

### Contextual properties

Properties added to the span macros are added to an ambient context and automatically included on any events emitted within that operation:

```
# use std::{thread, time::Duration};
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    emit::emit!("waiting a bit longer");

    thread::sleep(Duration::from_millis(sleep_ms));
}
```

```text
Event {
    module: "my_app",
    tpl: "waiting a bit longer",
    extent: Some(
        "2024-04-27T22:47:34.780288000Z",
    ),
    props: {
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-27T22:47:33.574839000Z".."2024-04-27T22:47:35.985844000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
```

Any operations started within a span will inherit its identifiers:

```
# use std::{thread, time::Duration};
#[emit::span("outer span", sleep_ms)]
fn outer_span(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    inner_span(sleep_ms / 2);
}

#[emit::span("inner span", sleep_ms)]
fn inner_span(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));
}
```

```text
Event {
    module: "my_app",
    tpl: "inner span",
    extent: Some(
        "2024-04-27T22:50:50.385706000Z".."2024-04-27T22:50:50.994509000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "inner span",
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_parent": 23995f85b4610391,
        "sleep_ms": 600,
        "span_id": fc8ed8f3a980609c,
    },
}
Event {
    module: "my_app",
    tpl: "outer span",
    extent: Some(
        "2024-04-27T22:50:49.180025000Z".."2024-04-27T22:50:50.994797000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "outer span",
        "sleep_ms": 1200,
        "span_id": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
    },
}
```

Notice the `span_parent` of `inner_span` is the same as the `span_id` of `outer_span`. That's because `inner_span` was called within the execution of `outer_span`.

### Propagating span context across threads

Ambient span properties are not shared across threads by default. This context needs to be fetched and sent across threads manually:

```
# use std::thread;
# fn my_operation() {}
thread::spawn({
    let ctxt = emit::Frame::current(emit::runtime::shared().ctxt());

    move || ctxt.call(|| {
        // Your code goes here
    })
});
```

This same process is also needed for async code that involves thread spawning:

```
# mod tokio { pub fn spawn(_: impl std::future::Future) {} }
tokio::spawn(
    emit::Frame::current(emit::runtime::shared().ctxt()).in_future(async {
        // Your code goes here
    }),
);
```

Async functions that simply migrate across threads in work-stealing runtimes don't need any manual work to keep their context across those threads.

### Propagating span context across services

`emit` doesn't implement any distributed trace propagation itself. This is the responsibility of end-users through their web framework and clients to manage.

When an incoming request arrives, you can parse the trace and span ids from its traceparent header and push them onto the current context:

```
// Parsed from a traceparent header
let trace_id = "12b2fde225aebfa6758ede9cac81bf4d";
let span_id = "23995f85b4610391";

let frame = emit::Frame::push(emit::runtime::shared().ctxt(), emit::props! {
    trace_id,
    span_id,
});

frame.call(handle_request);

#[emit::span("incoming request")]
fn handle_request() {
    // Your code goes here
}
```

```text
Event {
    module: "my_app",
    tpl: "incoming request",
    extent: Some(
        "2024-04-29T05:37:05.278488400Z".."2024-04-29T05:37:05.278636100Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "incoming request",
        "span_parent": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_id": 641a578cc05c9db2,
    },
}
```

This pattern of pushing the incoming traceparent onto the context and then immediately calling a span annotated function ensures the `span_id` parsed from the traceparent becomes the `span_parent` in the events emitted by your application, without emitting a span event for the calling service itself.

When making outbound requests, you can pull the current trace and span ids from the current context and format them into a traceparent header:

```
use emit::{well_known::{KEY_SPAN_ID, KEY_TRACE_ID}, Ctxt, Props};

let (trace_id, span_id) = emit::runtime::shared().ctxt().with_current(|props| {
    (
        props.pull::<emit::span::TraceId, _>(KEY_TRACE_ID),
        props.pull::<emit::span::SpanId, _>(KEY_SPAN_ID),
    )
});

if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
    let traceparent = format!("00-{trace_id}-{span_id}-00");

    // Push the traceparent header onto the request
}
```

### Completing spans manually

The `arg` control parameter can be applied to span macros to bind an identifier in the body of the annotated function for the [`Span`] that's created for it. This span can be completed manually, changing properties of the span along the way:

```
# use std::{thread, time::Duration};
#[emit::span(arg: span, "wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    if sleep_ms > 500 {
        span.complete_with(|span| {
            emit::warn!(
                event: span,
                when: emit::filter::always(),
                "wait a bit took too long",
            );
        });
    }
}

wait_a_bit(100);
wait_a_bit(1200);
```

```text
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-28T21:12:20.497595000Z".."2024-04-28T21:12:20.603108000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "trace_id": 5b9ab977a530dfa782eedd6db08fdb66,
        "sleep_ms": 100,
        "span_id": 6f21f5ddc707f730,
    },
}
Event {
    module: "my_app",
    tpl: "wait a bit took too long",
    extent: Some(
        "2024-04-28T21:12:20.603916000Z".."2024-04-28T21:12:21.808502000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "lvl": warn,
        "trace_id": 9abad69ac8bf6d6ef6ccde8453226aa3,
        "sleep_ms": 1200,
        "span_id": c63632332de89ac3,
    },
}
```

Take care when completing spans manually that they always match the configured filter. This can be done using the `when` control parameter like in the above example. If a span is created it _must_ be emitted, otherwise the resulting trace will be incomplete.

## Sampling metrics

Metrics are an effective approach to monitoring applications at scale. They can be cheap to collect, making them suitable for performance sensitive operations. They can also be compact to report, making them suitable for high-volume scenarios. `emit` doesn't provide any infrastructure for collecting or sampling metrics. What it does provide is a standard way to report metric samples as events.

A standard kind of metric is a monotonic counter, which can be represented as an atomic integer. In this example, our counter is for the number of bytes written to a file, which we'll call `bytes_written`. We can report a sample of this counter as an event by wrapping it in a [`Metric`]:

```
# fn sample_bytes_written() -> usize { 4643 }
use emit::{well_known::METRIC_AGG_COUNT, Clock};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let sample = sample_bytes_written();

rt.emit(
    emit::Metric::new(
        emit::module!(),
        now,
        "bytes_written",
        METRIC_AGG_COUNT,
        sample,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-29T10:08:24.780230000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 4643,
    },
}
```

Metrics may also be emitted manually:

```
# fn sample_bytes_written() -> usize { 4643 }
use emit::well_known::{EVENT_KIND_METRIC, METRIC_AGG_COUNT};

let sample = sample_bytes_written();

emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    event_kind: EVENT_KIND_METRIC,
    metric_agg: METRIC_AGG_COUNT,
    metric_name: "bytes_written",
    metric_value: sample,
);
```

The data model of metrics is an extension of `emit`'s events. Metric events are points or buckets in a timeseries. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following well-known properties:

- `event_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: A monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: A potentially non-monotonic sum of defined values.
    - `"min"`: The lowest ordered value.
    - `"max"`: The largest ordered value.
    - `"last"`: The most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.

### Cumulative metrics

Metric events with a point extent are cumulative. Their `metric_value` is the result of applying their `metric_agg` over the entire underlying stream up to that point.

The following metric reports the current number of bytes written as 591:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();

rt.emit(
    emit::Metric::new(
        emit::module!(),
        now,
        "bytes_written",
        METRIC_AGG_COUNT,
        591,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:53:41.069203000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 591,
    },
}
```

### Delta metrics

Metric events with a span extent are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

rt.emit(
    emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        17,
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:55:59.839770000Z".."2024-04-30T06:56:29.839770000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 17,
    },
}
```

### Timeseries metrics

Metric events with a span extent, where the `metric_value` is an array are a complete timeseries. Each element in the array is a bucket in the timeseries. The width of each bucket is the length of the extent divided by the number of buckets.

The following metric is for a timeseries with 15 buckets, where each bucket covers 1 second:

```
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let rt = emit::runtime::shared();

let now = rt.clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(15));

rt.emit(
    emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        &[
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
        emit::Empty,
    )
);
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T07:03:07.828185000Z".."2024-04-30T07:03:22.828185000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": [
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
    },
}
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

## Custom runtimes

All functionality in `emit` is based on a [`runtime::Runtime`]. When you call [`Setup::init`], it initializes the [`runtime::shared`] runtime for you, which is also what macros use by default.

You can implement your own runtime, providing your own implementations of the ambient clock, randomness, and global context. First, disable the default features of `emit` in your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"
default-features = false
features = ["std"]
```

This will ensure the `rt` control parameter is always passed to macros so that your custom runtime will always be used. Next, define your runtime itself and use it in macros:

```
// Define a static runtime to use
// In this example, we use the default implementations of most things,
// but you can also bring-your-own
static RUNTIME: emit::runtime::Runtime<
    MyEmitter,
    emit::Empty,
    emit::platform::thread_local_ctxt::ThreadLocalCtxt,
    emit::platform::system_clock::SystemClock,
    emit::platform::rand_rng::RandRng,
> = emit::runtime::Runtime::build(
    MyEmitter,
    emit::Empty,
    emit::platform::thread_local_ctxt::ThreadLocalCtxt::shared(),
    emit::platform::system_clock::SystemClock::new(),
    emit::platform::rand_rng::RandRng::new(),
);

struct MyEmitter;

impl emit::Emitter for MyEmitter {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        println!("{}", evt.to_event().msg());
    }

    fn blocking_flush(&self, _: std::time::Duration) {
        // Nothing to flush
    }
}

// Use your runtime with the `rt` control parameter
emit::emit!(rt: &RUNTIME, "emitted through a custom runtime");
```

```text
emitted through a custom runtime
```

## Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure to debug them:

```
# mod emit_term { pub fn stdout() -> impl emit::runtime::InternalEmitter { emit::runtime::AssertInternal(emit::emitter::from_fn(|_| {})) } }
fn main() {
    // Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
*/

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

pub use core::module_path as module;

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
pub mod platform;
pub mod span;
pub mod timer;

pub use self::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::Event, extent::Extent,
    filter::Filter, frame::Frame, kind::Kind, level::Level, metric::Metric, path::Path,
    props::Props, rng::Rng, span::Span, str::Str, template::Template, timer::Timer,
    timestamp::Timestamp, value::Value,
};

mod macro_hooks;

#[cfg(feature = "std")]
pub mod setup;
#[cfg(feature = "std")]
pub use setup::{setup, Setup};

#[doc(hidden)]
pub mod __private {
    pub extern crate core;
    pub use crate::macro_hooks::*;
}
