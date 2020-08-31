# `emit`

**Current Status: Proof-of-concept**

You'll need a current `nightly` to build this project.

This library is a playground for modern structured logging techniques for Rust, based on the work of `log` and `tracing`.

It's just a proof-of-concept that will need a lot more work to be polished into a consumable artifact, but sketches out a lot of design space.

## What does it look like?

Given a macro input like:

```rust
emit!("scheduling background work {description: work.description} ({id: work.id})", #[serde] work);
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

### `template`

An alternative to `format_args!` (and `log!`) for string templating. Templates take the following form:

```
pre_template_arg_1: FieldValue, pre_template_arg_n: FieldValue, template: Lit, post_template_arg_1: FieldValue, post_template_arg_n: FieldValue
```

where the `template` supports interpolating values with `{interpolated: FieldValue}`.

As an example, the following is a template:

```
log: some.log, debug, "This is the literal {interpolated: 42} of the template", extra
```

#### How are templates different from `format_args!`?

Templates differ from `format_args!` by using an explicit API for parsing at compile time. That way consumers can interrogate a parsed template directly instead of having to make assumptions about how it will be interpreted downstream just based on its input tokens.

Templates also use different syntax for interpolated values. Because they're just field values, templates support any arbitrary Rust expression between `{}` and have consistent syntax for associating an identifier with an expression. Templates don't invent flags for determining how to interpret expressions, that's left up to the caller to decide. In `emit`, we use attributes like `#[debug]` instead of flags like `?`.

#### How are templates different from `log!`?

Templates differ from `log!` by using field values for the inputs before and after the template. This works like named function arguments where arguments can be provided or omitted in any order. It's more future-proof than positional arguments without needing a lot more syntax for full structs.

### `value_bag`

The `ValueBag` type is the value API taken from `log` and wrapped up in its own crate. A `ValueBag` is an anonymous structured bag that supports casting, downcasting, formatting, and serializing. The goal of a `ValueBag` is to decouple the producers of structured data from its consumers. A `ValueBag` can _always_ be interrogated using the consumers serialization API of choice, even if that wasn't the one the producer used to capture the data in the first place.

Say we capture an `i32` using its `Display` implementation as a `ValueBag`:

```rust
let bag = ValueBag::capture_display(42);
```

That value can then be cast to a `u64`:

```rust
let num = bag.as_u64().unwrap();

assert_eq!(42, num);
```

It could also be serialized as a number using `serde`:

```rust
let num = serde_json::to_value(bag).unwrap();

assert!(num.is_number());
```

Say we derive `sval::Value` on a type and capture it as a `ValueBag`:

```rust
#[derive(Value)]
struct Work {
    id: u64,
    description: String,
}

let work = Work {
    id: 123,
    description: String::from("do the work"),
}

let bag = ValueBag::capture_sval(&work);
```

It could then be formatted using `Display`, even though `Work` never implemented that trait:

```rust
assert_eq!("Work { id: 123, description: \"do the work\" }", bag.to_string());
```

Or serialized using `serde` and retain its nested structure.

The tradeoff in all this is that `ValueBag` needs to depend on the serialization frameworks (`sval`, `serde`, and `std::fmt`) that it supports, instead of just providing an API of its own for others to plug into. Doing this lets `ValueBag` guarantee everything will always line up, and keep its own public API narrow. Each of these frameworks are stable though (except `sval` which is `1.0.0-alpha`).

### `macros`

`emit!` is implemented as a series of procedural macros. `emit!` itself is a function-like macro, and capturing modifiers like `#[debug]`, `#[display]`, `#[error]`, `#[sval]`, and `#[serde]` are implemented as attribute-like macros.

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
