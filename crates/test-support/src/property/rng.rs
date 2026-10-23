//! Small, reproducible SplitMix64 streams for tests, not cryptography.

/// A SplitMix64 generator whose output depends only on its seed and calls.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// Samples uniformly from `0..n`. Panics when `n` is zero.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "Rng::below requires a nonzero bound");
        let threshold = n.wrapping_neg() % n;
        loop {
            let value = self.u64();
            if value >= threshold {
                return value % n;
            }
        }
    }

    /// Samples an inclusive range, including the full i64 range.
    /// Panics when `lo > hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi, "Rng::range_i64 requires lo <= hi");
        let span = (i128::from(hi) - i128::from(lo)) as u64;
        let offset = if span == u64::MAX {
            self.u64()
        } else {
            self.below(span + 1)
        };
        (i128::from(lo) + i128::from(offset)) as i64
    }

    pub fn bool(&mut self) -> bool {
        self.u64() & 1 != 0
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.u64() as u8).collect()
    }

    /// Selects an element. Panics when the slice is empty.
    pub fn pick<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[self.below(values.len() as u64) as usize]
    }
}
