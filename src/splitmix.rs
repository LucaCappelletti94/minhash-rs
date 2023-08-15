//! Module for the SplitMix64 algorithm.
//!
//! # What is SplitMix64?
//! SplitMix64 is a fast, non-cryptographic, pseudo-random number generator.

pub trait SplitMix {
    fn splitmix(self) -> Self;
}

impl SplitMix for u64 {
    fn splitmix(self) -> Self {
        let mut z = self;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
