//! Module proving implementations of the XorShift algorithm for several words.
//!
//! # What is XorShift?
//! XorShift is a fast, non-cryptographic, pseudo-random number generator.
//! It is used in this crate to generate the permutations for the MinHash.

/// A value that can advance through an xorshift sequence.
pub trait XorShift {
    /// Returns the next value in the xorshift sequence.
    #[must_use]
    fn xorshift(self) -> Self;
}

impl XorShift for usize {
    fn xorshift(self) -> Self {
        (self as u64).xorshift() as usize
    }
}

impl XorShift for u64 {
    fn xorshift(self) -> Self {
        let mut x = self;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}

impl XorShift for u32 {
    fn xorshift(self) -> Self {
        let mut x = self;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    }
}

impl XorShift for u16 {
    fn xorshift(self) -> Self {
        u32::from(self).xorshift() as u16
    }
}

impl XorShift for u8 {
    fn xorshift(self) -> Self {
        let mut x = self;
        x ^= x << 3;
        x ^= x >> 7;
        x ^= x << 1;
        x
    }
}
