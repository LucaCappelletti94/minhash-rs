//! Trait providing the maximal value of a word type.
//!
//! A freshly created MinHash fills every word with this value, which acts as
//! the "empty" sentinel since any inserted hash compares smaller or equal.

/// A word type that can report its maximal value.
pub trait Maximal: Copy {
    /// Returns the largest representable value of this type.
    fn maximal() -> Self;
}

impl Maximal for u8 {
    fn maximal() -> Self {
        u8::MAX
    }
}

impl Maximal for u16 {
    fn maximal() -> Self {
        u16::MAX
    }
}

impl Maximal for u32 {
    fn maximal() -> Self {
        u32::MAX
    }
}

impl Maximal for u64 {
    fn maximal() -> Self {
        u64::MAX
    }
}

impl Maximal for usize {
    fn maximal() -> Self {
        usize::MAX
    }
}
