# `emit`

**Current Status: Proof-of-concept**

You'll need a current `nightly` to build this project.

This library is a playground for modern structured logging techniques for Rust, based on the work of `log` and `tracing`.

It's just a proof-of-concept that will need a lot more work to be polished into a consumable artifact, but sketches out a lot of design space.

## What does it look like?

Given a macro input like:

```rust
emit::info!("scheduling background work {description: work.description} ({id: work.id})", #[emit::as_serde] work);
```

the following output will be produced:

```
kvs (debug): [("description", upload all the documents), ("id", bbb1d632-4964-43ef-9883-7f4192f70c24), ("work", Work { id: "bbb1d632-4964-43ef-9883-7f4192f70c24", description: "upload all the documents", size: 1024 })]
kvs (json):  {"description":"upload all the documents","id":"bbb1d632-4964-43ef-9883-7f4192f70c24","work":{"id":"bbb1d632-4964-43ef-9883-7f4192f70c24","description":"upload all the documents","size":1024}}
msg:         scheduling background work `description` (`id`)
template:    scheduling background work upload all the documents (bbb1d632-4964-43ef-9883-7f4192f70c24)
```

along with an event in the current `tracing` subscriber:

```
Aug 31 16:07:39.358  INFO trybuild003: msg=scheduling background work upload all the documents (bbb1d632-4964-43ef-9883-7f4192f70c24) description=upload all the documents id=bbb1d632-4964-43ef-9883-7f4192f70c24 work=Work { id: "bbb1d632-4964-43ef-9883-7f4192f70c24", description: "upload all the documents", size: 1024 }
```

## Key pieces

The API is built around procedural macros that explicitly don't try to be backwards compatible with `format_args`. This is really just to keep the design space open.

`info!` is implemented as a series of procedural macros. `emit!` itself is a function-like macro, and capturing modifiers like `#[debug]`, `#[display]`, `#[error]`, `#[sval]`, and `#[serde]` are implemented as attribute-like macros.

The macro uses auto-ref to ensure owned and borrowed values are captured using the same syntax. It also expands to the same `match`-based expression as `format_args` so that short-lived inputs can still be captured.

There's also an environment variable, `EMIT_FILTER` that's used for compile-time filtering. This is different from `log` and `tracing`'s use of Cargo features for setting max levels, which interacts poorly with the way Cargo features are intended to be used. Setting a filter like:

```
EMIT_FILTER=my_crate
```

will compile `emit!` calls in any crate that isn't `my_crate` into no-ops. The implementation is _very_ simplistic, but enough to build a proper compile-time filtering implementation on top of.

## What's next?

Before this library would actually be useful it will need a lot of polish. I'd like to try keep it up to date with the `0.2` API of `tracing` as it evolves.

I'd also like to publish and stabilize the `ValueBag` API here and use it internally in `log` to finish up its `std::error` and `serde` support while also removing a lot of complexity from that crate.

In the meantime, I hope there's something interesting in here for anybody who's interested in structured logging for Rust!
