# foundation

This workspace contains two Rust crates:

| Crate | Type | Purpose |
| --- | --- | --- |
| `foundation` | library | Core utilities for arena allocation, parsing, reflection, serialization, templating, process execution, logging, and a small cooperative executor. |
| `foundation_derive` | proc-macro | Derive support for the reflection system in `foundation`, currently via `#[derive(Reflect)]` for structs. |

## Workspace layout

### `foundation`

`foundation` is the main library crate. Its public modules are:

- `alloc`: arena-backed allocation primitives, byte buffers, strings, string builders, and string pooling.
- `encoding`: thin wrappers for parsing JSON, YAML, and TOML input.
- `executor`: a deterministic single-threaded cooperative executor with shared state and `yield_now`.
- `file`: small filesystem helpers for loading, saving, listing, and mutating paths on Unix-like systems.
- `log`: minimal integration with the `log` crate using a stderr logger and level filtering.
- `process`: child-process helpers that collect stdin/stdout/stderr and integrate with the arena-backed string types.
- `reflect`: runtime type descriptions, an `Introspectable` trait, and a type registry.
- `serializer`: JSON serialization and deserialization built on top of the reflection metadata.
- `template`: a compact JSON-backed template engine with `{{path}}`, `{{#if}}`, and `{{#each}}`.
- `thread`: declarative helpers for configuring threads, handing work across threads, and running a simple worker pool.

This crate is the runtime half of the workspace. If you only need the utilities and reflection APIs, this is the crate to depend on.

### `foundation_derive`

`foundation_derive` is the procedural macro companion crate for `foundation`.

It currently provides:

- `#[derive(Reflect)]`: generates `foundation::reflect::Introspectable` implementations for named-field structs, tuple structs, and unit structs.

The generated reflection metadata records:

- the Rust type name
- `size_of::<Self>()`
- one reflected field entry per struct field
- byte offsets computed from the actual struct layout

Enums and unions are not currently supported by the derive macro.

## Relationship between the crates

`foundation` defines the reflection model and runtime APIs. `foundation_derive` exists to reduce the manual work required to implement that model for user-defined structs.
