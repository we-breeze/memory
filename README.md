# Breeze Memory

`memory` is the shared Rust memory subsystem for Breeze projects. It currently
owns the process allocator integration and allocation observability:

- a `mimalloc`-backed global allocator;
- cumulative and outstanding object/byte statistics;
- power-of-two layout buckets capped at 64 MiB;
- allocation-rate sampling by total bytes and per-bucket object rate.

The repository is a Cargo workspace with the primary library as its root
package. Future cohesive memory primitives such as arenas, slabs, or pools can
be added as modules first and split into workspace crates only when they have
independent consumers or dependency boundaries.

## Usage

```toml
[dependencies]
memory = { git = "https://github.com/we-breeze/memory.git", tag = "v0.0.1" }
```

Install the allocator exactly once in a binary:

```rust,no_run
memory::install_global_allocator!();

fn main() {
    let heap = memory::heap();
    println!("{heap:?}");
}
```

With the default `heap-stats` feature, `BrzMalloc` records size-bucketed
statistics and `RateStats` is available. With default features disabled,
`BrzMalloc` is a direct alias for `mimalloc::MiMalloc`, `heap()` returns `None`,
and statistics sampling APIs are not compiled.

## Verification

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo check --no-default-features
```

## Scope

This repository owns allocation, layout, reuse, and memory-lifecycle
primitives. Business caches and ordinary collections do not belong here merely
because their data resides in memory.
