# `antlog`

**Current Status: Still writing an initial implementation**

An experimental wrapper around the [`log`](https://docs.rs/log/0.4.6/log/) crate that exposes its structured features. The goal is to explore ways to leverage `log`'s structured logging to build more useful APIs on top.

It currently requires a `nightly` compiler.

The goals of this experiment are:

- To prove out the `log::kv` API and make sure it's suitable for capturing lots of different kinds of values.
- To explore alternative macros that aren't tied to `format_args`.
- To create a more structured default target for log events that includes other useful information like timestamps, raw templates, and correlation identifiers.
- To ensure zero-cost integration with `tracing` is possible and first-class.

## `macros`

See the `ui/pass` tests for some examples.

The `macros` crate is an alternative `log!` macro that's natively structured. It uses field value syntax within text templates to support interpolated key-value pairs. Templates with structured values are like an alternative `format_args!` that naturally support other capturing methods without needing to invent syntax for flags, and the rendering of late-bound and missing values.
