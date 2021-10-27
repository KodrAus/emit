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

### `tracing` integration

The `tracing` ecosystem has a lot of infrastructure supporting runtime logging, so it makes a lot of sense to make these front-end macros hook into `tracing`.

### Structure preserving field-value templates

The input to macros is a [field-value template](https://github.com/sval-rs/fv-template), which is a set of field-value pairs and a template literal, which can also contain field-value pairs between braces. Let's compare a simple example of a field-value template with a standard Rust format:

```rust
// field-value template
emit::info!("scheduling background work {description: work.description} ({id: work.id})", #[emit::as_serde] work);

// format args
format_args!("scheduling background work {} ({})", work.description, work.id);
```

We can't pass the `work` parameter in the format args version because it doesn't have a place in the template to get replaced into. This makes sense for format args, because its consumers are trying to produce a stream of text. If there were arguments to format that didn't have a place in the template then you wouldn't know how to include them in the text you're writing.

The other difference that jumps out is that field-value templates can interpolate expressions. That's a capability that format args [are also getting in a limited form](https://rust-lang.github.io/rfcs/2795-format-args-implicit-identifiers.html) though, so we could also write:

```rust
let description = work.description;
let id = work.id;

// field-value template
emit::info!("scheduling background work {description} ({id})");

// format args
format_args!("scheduling background work {description} ({id})");
```

Field-value templates stick to standard Rust syntax for defining what and how to interpolate so they use attributes instead of format flags. Let's say we want to capture using `Debug` instead of `Display`:

```rust
// field-value template
emit::info!("scheduling background work {#[emit::as_debug] my_value}");

// format args
format_args!("scheduling background work {my_value:?}");
```

The format flags are nicely compact, but can become difficult to read when you start to combine a lot of them (I can never remember how to left-pad a number with n `0`s). Field-value templates can use more field-values in attributes to tweak them further:

```rust
emit::info!("scheduling background work {#[emit::as_debug(capture: false)] my_value}");
```

If that starts to get a bit noisy in the template then they can be moved outside of it:

```rust
emit::info!("scheduling background work {my_value}", #[emit::as_debug(capture: false)] my_value);
```

Something that's more subtle is that format args capture their inputs using one of the `std::fmt` traits, like `Debug` or `Display`. These are focused on text formatting so aren't structure preserving. Field-value templates [capture a `ValueBag`](https://github.com/sval-rs/value-bag) that preserves its structure without any new traits that libraries need to implement.

That's the gist of it! Field-value templates lean on field-value syntax everywhere. Since nothing is position-dependent that means we can add new capabilities without affecting any existing users.

### Compile-time filtering

There's also an environment variable, `EMIT_FILTER` that's used for compile-time filtering. This is different from `log` and `tracing`'s use of Cargo features for setting max levels, which interacts poorly with the way Cargo features are intended to be used. Setting a filter like:

```
EMIT_FILTER=my_crate
```

will compile `emit` calls in any crate that isn't `my_crate` into no-ops. The implementation is _very_ simplistic, but enough to build a proper compile-time filtering implementation on top of.

## What's next?

Before this library would actually be useful it will need a lot of polish. I'd like to try keep it up to date with the `0.2` API of `tracing` as it evolves.

I'd also like to publish and stabilize the `ValueBag` API here and use it internally in `log` to finish up its `std::error` and `serde` support while also removing a lot of complexity from that crate.

In the meantime, I hope there's something interesting in here for anybody who's interested in structured logging for Rust!
