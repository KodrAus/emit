# `emit`

**Current Status: Proof-of-concept**

This library is a playground for modern structured logging techniques for Rust, based on the work of `log` and `tracing`.

It's just a proof-of-concept that will need a lot more work to be polished into a consumable artifact, but sketches out a lot of design space.

For some idea of what it can do, see the `tests/smoke-test` example.

## Structured logging

`emit` is a structured logging framework for manually instrumenting your applications with _events_.
An event is a point of change in a system surfaced to an observer along with rationale describing it.
Events are a model of your unique domain through the lens of significant interactions with it.

You can represent a lot of interesting observability signals using events.

```
emit::info!(extent: now, "A log record");
```

```
13:18:58.657 A log record
```

-----

```
emit::info!(extent: now..later, "A span");
```

```
13:19:03.657 5s A span
```

-----

```
emit::info!(extent: now, "An independent metric {metric_value: 1.0}");
```

```
13:18:58.657 An independent metric 1
```

-----

```
emit::info!(extent: now..later, "A cumulative metric {metric_value: 4.0}");
```

```
13:19:03.657 5s A cumulative metric 4
```

-----

```
emit::info!(extent: now..later, "A histogram metric {#[emit::as_serde] metric_value: [1.0, 3.0, 2.0, 5.0, 1.0]}");
```

```
13:19:03.657 5s A histogram metric (1, 3, 2, 5, 1)
▁▄▃▇▁
```

## How is this different?

`emit` takes a different path from `log` or `tracing` by abandoning `format_args!` as the basis of its instrumentation.
`format_args!` is the standard mechanism for building strings with interpolated values, but is geared towards constructing strings rather than capturing structured data.
`emit` defines a new templating syntax that is compatible with `format_args!` in simple cases, but is both more consistent and more capable in complex ones.

`emit` is focused just on structured logging.
It supports tracing implicitly through trace and span ids, but doesn't model traces or spans directly.
