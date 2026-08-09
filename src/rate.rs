//! Allocation-rate sampling from cumulative layout counters.

use std::time::{Duration, Instant};

use crate::{LayoutBuckets, NUM_BUCKETS, bucket_key, malloc::layout_buckets};

struct Snapshot {
    total_bytes: u64,
    alloc_objects: [u64; NUM_BUCKETS],
}

impl Snapshot {
    fn from(buckets: &LayoutBuckets) -> Self {
        let mut alloc_objects = [0; NUM_BUCKETS];
        let mut total_bytes = 0;
        for (index, value) in alloc_objects.iter_mut().enumerate() {
            *value = buckets.alloc_objects_at(index);
            total_bytes += *value * bucket_key(index) as u64;
        }
        Self {
            total_bytes,
            alloc_objects,
        }
    }
}

/// Stateful allocation-rate calculator.
pub struct RateStats {
    previous: Option<Snapshot>,
    previous_at: Option<Instant>,
}

impl RateStats {
    pub const fn new() -> Self {
        Self {
            previous: None,
            previous_at: None,
        }
    }

    /// Sample the process-wide allocator. The first call establishes a
    /// baseline and returns zero rates.
    pub fn sample(&mut self) -> Rate {
        self.sample_from(layout_buckets())
    }

    fn sample_from(&mut self, buckets: &LayoutBuckets) -> Rate {
        let now = Instant::now();
        let current = Snapshot::from(buckets);
        let rate = match (&self.previous, self.previous_at) {
            (Some(previous), Some(previous_at)) => {
                calculate_rate(previous, &current, now.duration_since(previous_at))
            }
            _ => Rate::zeroed(),
        };
        self.previous = Some(current);
        self.previous_at = Some(now);
        rate
    }
}

impl Default for RateStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation rates measured over one sampling interval.
#[derive(Clone, Debug, Default)]
pub struct Rate {
    pub bps: u64,
    pub ops: u64,
    pub per_bucket: Vec<BucketRate>,
}

impl Rate {
    fn zeroed() -> Self {
        Self {
            bps: 0,
            ops: 0,
            per_bucket: (0..NUM_BUCKETS)
                .map(|index| BucketRate {
                    key: bucket_key(index),
                    qps: 0,
                })
                .collect(),
        }
    }

    pub fn active_buckets(&self) -> impl Iterator<Item = &BucketRate> {
        self.per_bucket.iter().filter(|bucket| bucket.qps > 0)
    }
}

/// Object allocations per second for one size bucket.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BucketRate {
    pub key: usize,
    pub qps: u64,
}

fn calculate_rate(previous: &Snapshot, current: &Snapshot, elapsed: Duration) -> Rate {
    if elapsed.is_zero() {
        return Rate::zeroed();
    }
    let per_bucket = (0..NUM_BUCKETS)
        .map(|index| BucketRate {
            key: bucket_key(index),
            qps: per_second(
                current.alloc_objects[index].saturating_sub(previous.alloc_objects[index]),
                elapsed,
            ),
        })
        .collect::<Vec<_>>();
    Rate {
        bps: per_second(
            current.total_bytes.saturating_sub(previous.total_bytes),
            elapsed,
        ),
        ops: per_bucket.iter().map(|bucket| bucket.qps).sum(),
        per_bucket,
    }
}

fn per_second(delta: u64, elapsed: Duration) -> u64 {
    const NANOS_PER_SECOND: u128 = 1_000_000_000;
    let scaled = u128::from(delta).saturating_mul(NANOS_PER_SECOND) / elapsed.as_nanos();
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn first_sample_is_zero_and_next_sample_uses_delta() {
        let buckets = LayoutBuckets::new();
        let mut sampler = RateStats::new();
        assert_eq!(sampler.sample_from(&buckets).bps, 0);

        sleep(Duration::from_millis(20));
        buckets.record_alloc(1024);
        buckets.record_alloc(8);
        let rate = sampler.sample_from(&buckets);
        assert!(rate.bps > 0);
        assert!(rate.ops > 0);
        assert_eq!(rate.active_buckets().count(), 2);
    }

    #[test]
    fn deallocation_does_not_increase_allocation_rate() {
        let buckets = LayoutBuckets::new();
        let mut sampler = RateStats::new();
        sampler.sample_from(&buckets);
        buckets.record_alloc(2048);
        sleep(Duration::from_millis(20));
        assert!(sampler.sample_from(&buckets).bps > 0);

        buckets.record_dealloc(2048);
        sleep(Duration::from_millis(20));
        assert_eq!(sampler.sample_from(&buckets).bps, 0);
    }
}
