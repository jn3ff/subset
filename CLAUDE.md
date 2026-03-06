# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # build entire workspace
cargo test                           # run all tests
cargo test -p subset                 # run tests for the main crate only
cargo test -p subset <test_name>     # run a single test by name
```

## Architecture

This is a Rust workspace containing a proc-macro derive crate for generating `From<T>` impls between structs that share fields (struct-to-struct projection).

**Two crates:**
- `subset/` — public-facing library. Re-exports the `Subset` derive macro from `subset-derive` and defines the `Subset<T>: From<T>` trait.
- `subset-derive/` — proc-macro crate. Contains the actual derive macro implementation.

**How the derive macro works** (`subset-derive/src/subset.rs`):
1. Parses the `#[subset(from = "SourceType")]` struct-level attribute to get the source type.
2. Iterates fields of the target struct. For each field, determines the RHS of the assignment via `field_rhs_tokens()`:
   - Default: `source.<field_name>` (same-named field)
   - `#[subset(alias = "...")]`: maps from a differently-named source field
   - `#[subset(path = "a.b.c")]`: maps from a nested field via chained access
3. Emits a `From<SourceType>` impl and a `Subset<SourceType>` trait impl.

**Tests** are integration tests in `subset/tests/`, one per feature: `basic.rs`, `alias.rs`, `path.rs`, `everything.rs`.

## Key Constraints

- Rust edition 2024, resolver 3
- Only supports named-field structs (no tuple structs, no enums)
- `chrono` is a dev-dependency used only in tests
