//! Per-session pool of cleared `String` buffers recycled across frames.
//!
//! Text recorded into commands ends up owned by the layout tree. When the
//! tree is recycled ([`super::LayoutArena::recycle`]) its strings are cleared
//! and parked here, and the next frame's widgets ([`crate::Context`]) take
//! them back to format their rows into, so a steady-state frame of a list,
//! table, separator or progress bar makes no allocation for its own text.
//!
//! Strings are bucketed by capacity class (`2^class` bytes) so a take never
//! hands back a buffer too small for the requested length: every string in
//! bucket `c` holds at least `2^c` bytes. The pool is bounded both per bucket
//! and in total retained bytes, so a frame that briefly records a lot of text
//! does not pin that memory for the rest of the session.

/// Smallest class: requests below 16 bytes share one bucket.
const MIN_CLASS: u32 = 4;
/// Largest class kept (64 KiB). Bigger strings are dropped on return.
const MAX_CLASS: u32 = 16;
const BUCKETS: usize = (MAX_CLASS - MIN_CLASS + 1) as usize;
/// Strings kept per bucket.
const MAX_PER_BUCKET: usize = 256;
/// Total capacity kept across all buckets.
const MAX_RETAINED_BYTES: usize = 512 * 1024;
/// Entries of the bucket below a request's class checked for a fit.
const LOWER_SCAN: usize = 4;
/// Larger classes a request may take from before allocating.
const UPPER_CLASSES: usize = 2;

#[derive(Debug, Default)]
pub(crate) struct StringPool {
    buckets: [Vec<String>; BUCKETS],
    retained_bytes: usize,
}

/// Bucket index whose strings all hold at least `cap` bytes, rounding up.
#[inline]
fn class_for_request(cap: usize) -> Option<usize> {
    let class = cap
        .max(1)
        .next_power_of_two()
        .trailing_zeros()
        .max(MIN_CLASS);
    (class <= MAX_CLASS).then(|| (class - MIN_CLASS) as usize)
}

/// Bucket index for a returned string of capacity `cap`, rounding down.
#[inline]
fn class_for_capacity(cap: usize) -> Option<usize> {
    if cap < (1 << MIN_CLASS) {
        // Below the smallest class: too small to be worth keeping.
        return None;
    }
    let class = usize::BITS - 1 - cap.leading_zeros();
    (class <= MAX_CLASS).then(|| (class - MIN_CLASS) as usize)
}

impl StringPool {
    /// An empty string with capacity for at least `len` bytes, reused from
    /// the pool when one is available.
    ///
    /// A miss allocates a power-of-two capacity, so the buffer comes back
    /// into exactly the bucket the same request looks in next frame.
    #[inline]
    pub(crate) fn take(&mut self, len: usize) -> String {
        let Some(first) = class_for_request(len) else {
            return String::with_capacity(len);
        };
        // An exactly sized string (from user text, say) rounds down into the
        // bucket below; take it when it is big enough.
        if first > 0 {
            let below = &mut self.buckets[first - 1];
            let scan = below.len().saturating_sub(LOWER_SCAN);
            if let Some(pos) = below[scan..].iter().rposition(|s| s.capacity() >= len) {
                let s = below.swap_remove(scan + pos);
                self.retained_bytes -= s.capacity();
                return s;
            }
        }
        // Up to UPPER_CLASSES larger classes, so a short request never pins
        // a much larger buffer that a later request needs.
        let last = (first + UPPER_CLASSES).min(BUCKETS - 1);
        for bucket in &mut self.buckets[first..=last] {
            if let Some(s) = bucket.pop() {
                self.retained_bytes -= s.capacity();
                return s;
            }
        }
        String::with_capacity(1 << (first as u32 + MIN_CLASS))
    }

    /// Park `s` (its contents are discarded) for a later [`take`](Self::take).
    #[inline]
    pub(crate) fn put(&mut self, mut s: String) {
        let cap = s.capacity();
        let Some(class) = class_for_capacity(cap) else {
            return;
        };
        let bucket = &mut self.buckets[class];
        if bucket.len() >= MAX_PER_BUCKET || self.retained_bytes + cap > MAX_RETAINED_BYTES {
            return;
        }
        s.clear();
        self.retained_bytes += cap;
        bucket.push(s);
    }

    /// Strings currently pooled.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.buckets.iter().map(Vec::len).sum()
    }

    /// Total capacity currently pooled, in bytes.
    #[cfg(test)]
    pub(crate) fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_never_returns_a_smaller_buffer_than_requested() {
        let mut pool = StringPool::default();
        for cap in [16, 17, 31, 32, 100, 600, 4096, 5000] {
            pool.put(String::with_capacity(cap));
        }
        for len in [0, 1, 15, 16, 17, 33, 100, 513, 600, 4000, 4097] {
            let s = pool.take(len);
            assert!(s.is_empty());
            assert!(
                s.capacity() >= len,
                "take({len}) returned capacity {}",
                s.capacity()
            );
        }
    }

    #[test]
    fn returned_strings_are_cleared_and_reused() {
        let mut pool = StringPool::default();
        let mut s = String::with_capacity(64);
        s.push_str("previous frame");
        let ptr = s.as_ptr();
        pool.put(s);
        let again = pool.take(20);
        assert!(again.is_empty());
        assert_eq!(again.as_ptr(), ptr, "the pooled buffer should be reused");
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.retained_bytes(), 0);
    }

    #[test]
    fn pool_is_bounded() {
        let mut pool = StringPool::default();
        for _ in 0..10_000 {
            pool.put(String::with_capacity(32));
        }
        assert_eq!(pool.len(), MAX_PER_BUCKET);
        for _ in 0..1_000 {
            pool.put(String::with_capacity(8 * 1024));
        }
        assert!(pool.retained_bytes() <= MAX_RETAINED_BYTES);
        pool.put(String::with_capacity(1 << 20));
        assert!(pool.retained_bytes() <= MAX_RETAINED_BYTES);
        // Tiny buffers are not worth keeping.
        let before = pool.len();
        pool.put(String::with_capacity(4));
        assert_eq!(pool.len(), before);
    }
}
