//! Conversion of `u64` hash values into the narrower MinHash word types.

/// Conversion from a `u64` hash value into a narrower MinHash word.
///
/// The hash generator always produces `u64` values (from the hasher and
/// SplitMix), so the only conversions the crate needs are from `u64` into each
/// supported word type. The bound used throughout the crate is
/// `u64: Primitive<Word>`.
pub trait Primitive<T> {
    /// Converts `self` into the target word type, narrowing if necessary.
    fn convert(self) -> T;
}

impl Primitive<u8> for u64 {
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for u64 {
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for u64 {
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for u64 {
    fn convert(self) -> u64 {
        self
    }
}

impl Primitive<usize> for u64 {
    fn convert(self) -> usize {
        self as usize
    }
}

/// Convert a word type to `u64` for sparse digest decoding.
///
/// Sparse mode stores encoded digests as `Word` values; this trait provides
/// the reverse conversion needed to decode them back to `u64`.
pub trait ToU64 {
    /// Convert to `u64`.
    fn to_u64(self) -> u64;
}

impl ToU64 for u64 {
    #[inline]
    fn to_u64(self) -> u64 {
        self
    }
}

impl ToU64 for usize {
    #[inline]
    fn to_u64(self) -> u64 {
        self as u64
    }
}

impl ToU64 for u8 {
    #[inline]
    fn to_u64(self) -> u64 {
        u64::from(self)
    }
}

impl ToU64 for u16 {
    #[inline]
    fn to_u64(self) -> u64 {
        u64::from(self)
    }
}

impl ToU64 for u32 {
    #[inline]
    fn to_u64(self) -> u64 {
        u64::from(self)
    }
}
/// Marker trait for word types that support sparse mode.
///
/// Only `u64` and `usize` implement this, since sparse mode stores full
/// `u64` SipHash/FNV digests. Narrow types (`u8`, `u16`, `u32`) would
/// truncate digests and break injectivity.
pub trait SparseWord: ToU64 {
    /// Convert a `u64` digest back to this word type without loss.
    fn from_digest(digest: u64) -> Self;
}

impl SparseWord for u64 {
    #[inline]
    fn from_digest(digest: u64) -> Self {
        digest
    }
}

#[cfg(target_pointer_width = "64")]
impl SparseWord for usize {
    #[inline]
    fn from_digest(digest: u64) -> Self {
        digest as usize
    }
}
