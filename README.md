# `antlog`

An experimental wrapper around the [`log`](https://docs.rs/log/0.4.6/log/) crate that exposes its structured features. The goal is to explore ways to leverage `log`'s structured logging to build more useful APIs on top.

It currently requires a `nightly` compiler.

## Macros

The `macros` crate is an alternative `log!` macro that's natively structured.

## Enrichment

The `enrich` crate is an implementation of contextual logging where ambient properties exist within scopes. Typically these scopes will cover transactions like handled web requests.