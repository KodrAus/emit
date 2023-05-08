# `emit`

**Current Status: Proof-of-concept**

This library is a playground for modern structured logging techniques for Rust, based on the work of `log` and `tracing`.

It's just a proof-of-concept that will need a lot more work to be polished into a consumable artifact, but sketches out a lot of design space.

For some idea of what it can do, see the `tests/smoke-test` example.

## Structured logging

`emit` is a structured logging framework for manually instrumenting your applications with _events_.
An event is a point of change in a system surfaced to an observer along with rationale describing it.
Events are a model of your unique domain through the lens of significant interactions with it.

## How is this different?

`emit` takes a different path from `log` or `tracing` by abandoning `format_args!` as the basis of its instrumentation.
`format_args!` is the standard mechanism for building strings with interpolated values, but is geared towards constructing strings rather than capturing structured data.
`emit` defines a new templating syntax that is compatible with `format_args!` in simple cases, but is both more consistent and more capable in complex ones.

`emit` is focused just on structured logging.
It supports tracing implicitly through trace and span ids, but doesn't model traces or spans directly.
