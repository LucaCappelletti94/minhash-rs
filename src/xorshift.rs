//! XorShift pseudo-random step used by the MinHash permutation stream.
//!
//! # What is XorShift?
//! XorShift is a fast, non-cryptographic step used to advance a value through
//! a full-period sequence over the non-zero space. MinHash uses it to derive
//! the per-permutation hash stream from the raw digest.
//!
//! The trait is implemented only for the [`HashType`](crate::hashtype::HashType)
//! widths the crate supports as an internal hash stream: [`u64`] and [`u32`].
//! `Word` types (`u8`, `u16`, `u32`, `u64`, `usize`) do not need an
//! `XorShift` impl because the stream lives entirely on the `Hash` side.

/// A value that can advance through an xorshift sequence.
pub trait XorShift {
    /// Returns the next value in the xorshift sequence.
    #[must_use]
    fn xorshift(self) -> Self;
}

impl XorShift for u64 {
    #[inline]
    fn xorshift(self) -> Self {
        let mut x = self;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}

impl XorShift for u32 {
    #[inline]
    fn xorshift(self) -> Self {
        let mut x = self;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    }
}
