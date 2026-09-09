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
memory = { package = "brz-memory", version = "0.0.2" }
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

## Releases

CI runs formatting, Clippy, and tests. To publish, open **Actions → Publish → Run workflow** on `main`. Leave `retry_tag` empty to allocate the next `v0.0.x` tag. The workflow validates the code, commits the version, pushes the commit and tag atomically, and publishes to crates.io using the organization secret `CARGO_REGISTRY_TOKEN`.

If publication fails after the tag was pushed, rerun with that existing tag in `retry_tag`. A normal push or pull request does not publish. Historical tags retain their original version numbers; use new release tags for registry packages.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
