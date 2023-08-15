//! Module proving implementations of the XorShift algorithm for several words.
//!
//! # What is XorShift?
//! XorShift is a fast, non-cryptographic, pseudo-random number generator.
//! It is used in this crate to generate the permutations for the MinHash.

pub trait XorShift {
    /// Returns the next value in the xorshift sequence.
    fn xorshift(&mut self) -> Self;
}

impl XorShift for usize {
    fn xorshift(&mut self) -> Self {
        (*self as u64).xorshift() as usize
    }
}

impl XorShift for u64 {
    fn xorshift(&mut self) -> Self {
        *self ^= *self << 13;
        *self ^= *self >> 7;
        *self ^= *self << 17;
        *self
    }
}

impl XorShift for u32 {
    fn xorshift(&mut self) -> Self {
        *self ^= *self << 13;
        *self ^= *self >> 17;
        *self ^= *self << 5;
        *self
    }
}

impl XorShift for u16 {
    fn xorshift(&mut self) -> Self {
        (*self as u32).xorshift() as u16
    }
}

impl XorShift for u8 {
    fn xorshift(&mut self) -> Self {
        *self ^= *self << 3;
        *self ^= *self >> 7;
        *self ^= *self << 1;
        *self
    }
}
