//! Global mimalloc wrapper with optional allocation accounting.

#[cfg(feature = "heap-stats")]
use crate::stats::LayoutBuckets;

/// Aggregate heap statistics for external reporting.
#[derive(Clone, Copy, Debug, Default)]
pub struct HeapStats {
    /// Total bytes allocated, cumulatively.
    pub total: u64,
    /// Outstanding bytes: allocated minus freed.
    pub used: u64,
    /// Total objects allocated, cumulatively.
    pub total_objects: u64,
    /// Outstanding objects: allocated minus freed.
    pub used_objects: u64,
}

#[cfg(feature = "heap-stats")]
mod stats_enabled {
    use std::alloc::{GlobalAlloc, Layout};

    use mimalloc::MiMalloc;

    use super::{HeapStats, LayoutBuckets};

    static BUCKETS: LayoutBuckets = LayoutBuckets::new();

    /// Global allocator delegating to mimalloc while recording layout buckets.
    pub struct BrzMalloc;

    // SAFETY: every operation is delegated to `MiMalloc` with the original
    // pointer and layout. The accounting side effect uses only relaxed atomic
    // counters and does not dereference or retain allocator-owned pointers.
    unsafe impl GlobalAlloc for BrzMalloc {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            BUCKETS.record_alloc(layout.size());
            // SAFETY: the caller upholds `GlobalAlloc::alloc`'s layout contract.
            unsafe { MiMalloc.alloc(layout) }
        }

        #[inline]
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            BUCKETS.record_dealloc(layout.size());
            // SAFETY: the caller guarantees `ptr` was allocated with this
            // allocator and that `layout` matches that allocation.
            unsafe { MiMalloc.dealloc(ptr, layout) }
        }
    }

    /// Snapshot the global heap statistics.
    pub fn heap() -> Option<HeapStats> {
        Some(HeapStats {
            total: BUCKETS.total_bytes(),
            used: BUCKETS.used_bytes(),
            total_objects: BUCKETS.total_objects(),
            used_objects: BUCKETS.used_objects(),
        })
    }

    /// Borrow the global layout buckets for detailed reporting.
    pub fn layout_buckets() -> &'static LayoutBuckets {
        &BUCKETS
    }
}

#[cfg(feature = "heap-stats")]
pub use stats_enabled::{BrzMalloc, heap, layout_buckets};

#[cfg(not(feature = "heap-stats"))]
mod stats_disabled {
    use super::HeapStats;

    pub type BrzMalloc = mimalloc::MiMalloc;

    /// Heap statistics are unavailable when `heap-stats` is disabled.
    pub const fn heap() -> Option<HeapStats> {
        None
    }
}

#[cfg(not(feature = "heap-stats"))]
pub use stats_disabled::{BrzMalloc, heap};
