/// Conversion from a `u64` hash value into a narrower MinHash word.
///
/// The hash generator always produces `u64` values (from the hasher and
/// SplitMix), so the only conversions the crate needs are from `u64` into each
/// supported word type. The bound used throughout the crate is
/// `u64: Primitive<Word>`.
pub trait Primitive<T> {
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
