//! Module for the SplitMix64 algorithm.
//!
//! # What is SplitMix64?
//! SplitMix64 is a fast, non-cryptographic, pseudo-random number generator.

/// A value that can be mixed with the SplitMix64 finalizer.
pub trait SplitMix {
    /// Returns the SplitMix64 mix of `self`.
    #[must_use]
    fn splitmix(self) -> Self;
}

impl SplitMix for u64 {
    fn splitmix(self) -> Self {
        let mut z = self;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}
