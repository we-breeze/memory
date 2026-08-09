//! Power-of-two size buckets for allocation counters.

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use crossbeam_utils::CachePadded;

const ZERO_BUCKET: usize = 0;
const FIRST_POW2_INDEX: usize = 1;
const MAX_POW2: usize = 1 << 26;

/// One zero-size bucket plus buckets for `2^0..=2^26`.
pub const NUM_BUCKETS: usize = FIRST_POW2_INDEX + 27;

/// Map an allocation size to its power-of-two bucket index.
#[inline]
pub fn bucket_index(size: usize) -> usize {
    if size == 0 {
        ZERO_BUCKET
    } else {
        pow2_bucket_index(round_up_pow2(size))
    }
}

#[inline]
fn pow2_bucket_index(key: usize) -> usize {
    debug_assert!(key.is_power_of_two());
    let index = (key.trailing_zeros() as usize) + FIRST_POW2_INDEX;
    index.min(NUM_BUCKETS - 1)
}

#[inline]
fn round_up_pow2(value: usize) -> usize {
    if value > MAX_POW2 {
        MAX_POW2
    } else if value.is_power_of_two() {
        value
    } else {
        value.next_power_of_two()
    }
}

/// Return the size represented by `index`.
#[inline]
pub fn bucket_key(index: usize) -> usize {
    if index == ZERO_BUCKET {
        0
    } else {
        let exponent = index - FIRST_POW2_INDEX;
        if exponent >= 26 {
            MAX_POW2
        } else {
            1usize << exponent
        }
    }
}

type Cell = CachePadded<AtomicU64>;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO_CELL: Cell = CachePadded::new(AtomicU64::new(0));

/// Allocation and deallocation object counts grouped by layout size.
///
/// Counters use relaxed atomics because they are observations rather than
/// synchronization. A full snapshot is consequently not transactional.
pub struct LayoutBuckets {
    alloc_objects: [Cell; NUM_BUCKETS],
    free_objects: [Cell; NUM_BUCKETS],
}

impl LayoutBuckets {
    /// Construct zeroed counters without runtime initialization.
    pub const fn new() -> Self {
        Self {
            alloc_objects: [ZERO_CELL; NUM_BUCKETS],
            free_objects: [ZERO_CELL; NUM_BUCKETS],
        }
    }

    #[inline]
    pub fn record_alloc(&self, size: usize) {
        self.alloc_objects[bucket_index(size)].fetch_add(1, Relaxed);
    }

    #[inline]
    pub fn record_dealloc(&self, size: usize) {
        self.free_objects[bucket_index(size)].fetch_add(1, Relaxed);
    }

    #[inline]
    pub fn alloc_objects_at(&self, index: usize) -> u64 {
        self.alloc_objects[index].load(Relaxed)
    }

    #[inline]
    pub fn free_objects_at(&self, index: usize) -> u64 {
        self.free_objects[index].load(Relaxed)
    }

    pub fn total_objects(&self) -> u64 {
        (0..NUM_BUCKETS)
            .map(|index| self.alloc_objects[index].load(Relaxed))
            .sum()
    }

    pub fn used_objects(&self) -> u64 {
        (0..NUM_BUCKETS)
            .map(|index| {
                self.alloc_objects[index]
                    .load(Relaxed)
                    .saturating_sub(self.free_objects[index].load(Relaxed))
            })
            .sum()
    }

    pub fn total_bytes(&self) -> u64 {
        (0..NUM_BUCKETS)
            .map(|index| self.alloc_objects[index].load(Relaxed) * bucket_key(index) as u64)
            .sum()
    }

    pub fn used_bytes(&self) -> u64 {
        (0..NUM_BUCKETS)
            .map(|index| {
                self.alloc_objects[index]
                    .load(Relaxed)
                    .saturating_sub(self.free_objects[index].load(Relaxed))
                    * bucket_key(index) as u64
            })
            .sum()
    }

    /// Copy every bucket in ascending-key order.
    pub fn snapshot(&self) -> Vec<BucketSnapshot> {
        (0..NUM_BUCKETS)
            .map(|index| {
                let objects = self.alloc_objects[index]
                    .load(Relaxed)
                    .saturating_sub(self.free_objects[index].load(Relaxed));
                BucketSnapshot {
                    key: bucket_key(index),
                    objects,
                    bytes: objects * bucket_key(index) as u64,
                }
            })
            .collect()
    }
}

impl Default for LayoutBuckets {
    fn default() -> Self {
        Self::new()
    }
}

/// Point-in-time view of one layout bucket.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BucketSnapshot {
    pub key: usize,
    pub objects: u64,
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_sizes_and_caps_oversized_allocations() {
        assert_eq!(bucket_key(bucket_index(0)), 0);
        assert_eq!(bucket_key(bucket_index(1)), 1);
        assert_eq!(bucket_key(bucket_index(3)), 4);
        assert_eq!(bucket_key(bucket_index(513)), 1024);
        assert_eq!(bucket_key(bucket_index(usize::MAX)), MAX_POW2);
        assert_eq!(NUM_BUCKETS, 28);
    }

    #[test]
    fn records_cumulative_and_outstanding_totals() {
        let buckets = LayoutBuckets::new();
        buckets.record_alloc(511);
        buckets.record_alloc(513);
        buckets.record_alloc(513);
        assert_eq!(buckets.total_objects(), 3);
        assert_eq!(buckets.total_bytes(), 512 + 1024 + 1024);

        buckets.record_dealloc(513);
        assert_eq!(buckets.used_objects(), 2);
        assert_eq!(buckets.used_bytes(), 512 + 1024);
        assert_eq!(buckets.total_objects(), 3);
    }

    #[test]
    fn over_free_saturates_without_wrapping() {
        let buckets = LayoutBuckets::new();
        buckets.record_alloc(513);
        buckets.record_dealloc(513);
        buckets.record_dealloc(513);
        assert_eq!(buckets.used_objects(), 0);
        assert_eq!(buckets.used_bytes(), 0);
    }

    #[test]
    fn snapshot_is_ordered_and_bucketed() {
        let buckets = LayoutBuckets::new();
        buckets.record_alloc(7);
        buckets.record_alloc(513);
        let snapshot = buckets.snapshot();
        assert!(snapshot.windows(2).all(|pair| pair[0].key < pair[1].key));
        assert_eq!(snapshot.iter().find(|item| item.key == 8).unwrap().bytes, 8);
        assert_eq!(
            snapshot.iter().find(|item| item.key == 1024).unwrap().bytes,
            1024
        );
    }
}
