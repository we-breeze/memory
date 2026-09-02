//! Breeze memory subsystem.
//!
//! The crate currently provides [`BrzMalloc`], a global allocator wrapper
//! around mimalloc, plus size-bucketed allocation statistics and allocation
//! rate sampling. Future memory-owned primitives such as arenas, slabs, and
//! pools can be added behind cohesive modules without changing consumers.
//!
//! Install the allocator once from a binary crate:
//!
//! ```no_run
//! memory::install_global_allocator!();
//! ```

mod malloc;
#[cfg(feature = "heap-stats")]
mod rate;
mod stats;

#[cfg(feature = "heap-stats")]
pub use malloc::layout_buckets;
pub use malloc::{BrzMalloc, HeapStats, heap};
#[cfg(feature = "heap-stats")]
pub use rate::{BucketRate, Rate, RateStats};
pub use stats::{BucketSnapshot, LayoutBuckets, NUM_BUCKETS, bucket_index, bucket_key};

/// Construct the configured allocator for the installation macro.
///
/// This is public only because exported macros expand in the consumer crate.
#[doc(hidden)]
#[cfg(feature = "heap-stats")]
pub const fn __allocator() -> BrzMalloc {
    BrzMalloc
}

/// Construct the configured allocator for the installation macro.
#[doc(hidden)]
#[cfg(not(feature = "heap-stats"))]
pub const fn __allocator() -> BrzMalloc {
    mimalloc::MiMalloc
}

/// Install [`BrzMalloc`] as the process-wide global allocator.
///
/// Call this exactly once at the top level of a binary. The `unsafe impl
/// GlobalAlloc` remains inside this crate, so consumers do not need to allow
/// unsafe code.
#[macro_export]
macro_rules! install_global_allocator {
    () => {
        #[global_allocator]
        static _BRZ_MALLOC_GLOBAL: $crate::BrzMalloc = $crate::__allocator();
    };
}
