# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # build entire workspace
cargo test                           # run all tests
cargo test -p subset                 # run tests for the main crate only
cargo test -p subset <test_name>     # run a single test by name
cargo test -p subset --features functions  # run tests including functions feature
```

## Architecture

This is a Rust workspace containing a proc-macro derive crate for generating `From<T>` impls between structs that share fields (struct-to-struct projection).

**Two crates:**
- `subset/` — public-facing library. Re-exports the `Subset` derive macro from `subset-derive` and defines the `Subset<T>: From<T>` trait.
- `subset-derive/` — proc-macro crate. Contains the actual derive macro implementation.

**How the derive macro works** (`subset-derive/src/subset.rs`):
1. Parses struct-level `#[subset(...)]` attributes: `from` (required source type) and `functions` (optional, requires feature).
2. Iterates fields of the target struct. For each field, `field_rhs_tokens()` determines the RHS:
   - Default: `from.<field_name>` (same-named field)
   - `#[subset(alias = "...")]`: maps from a differently-named source field
   - `#[subset(path = "a.b.c")]`: maps from a nested field via chained access
   - `#[subset(generate = "expr")]`: arbitrary expression using `from` as the source binding
   - Only one of `alias`, `path`, or `generate` may be set per field.
3. Emits a `From<SourceType>` impl and a `Subset<SourceType>` trait impl.

**`functions` feature** (gated behind `features = ["functions"]`):
- Allows `#[subset(functions = "method_name")]` or `#[subset(functions = ["a", "b"])]` on the struct to copy and rewrite methods from the source type onto the subset struct.
- `registry.rs` — scans `.rs` files at compile time to find inherent `impl` methods on the source type. Uses a thread-local cache for derived methods and file contents.
- `rewrite.rs` — `FieldRewriter` (implements `VisitMut`) rewrites `self.field` accesses in copied method bodies using the reverse field mapping (alias/path-aware).
- Derivative support: rewritten methods are registered so downstream subsets can chain off earlier subsets.

**Tests** are integration tests in `subset/tests/`, one per feature: `basic.rs`, `alias.rs`, `path.rs`, `generate.rs`, `reference.rs`, `functions-simple.rs`, `functions-alias.rs`, `functions-path.rs`, `functions-derivative.rs`.

## Key Constraints

- Rust edition 2024, resolver 3
- Only supports named-field structs (no tuple structs, no enums)
- `chrono` is a dev-dependency used only in tests
- The source variable in generated `From` impls is named `from` (not `source` or `value`)
