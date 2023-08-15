pub trait Maximal: Copy {
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

impl Maximal for u128 {
    fn maximal() -> Self {
        u128::MAX
    }
}
