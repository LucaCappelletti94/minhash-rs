pub trait Primitive<T> {
    fn convert(self) -> T;
}

impl Primitive<u8> for u8 {
    fn convert(self) -> u8 {
        self
    }
}

impl Primitive<u16> for u8 {
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for u8 {
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for u8 {
    fn convert(self) -> u64 {
        self as u64
    }
}

impl Primitive<u8> for u16 {
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for u16 {
    fn convert(self) -> u16 {
        self
    }
}

impl Primitive<u32> for u16 {
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for u16 {
    fn convert(self) -> u64 {
        self as u64
    }
}

impl Primitive<u8> for u32 {
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for u32 {
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for u32 {
    fn convert(self) -> u32 {
        self
    }
}

impl Primitive<u64> for u32 {
    fn convert(self) -> u64 {
        self as u64
    }
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

impl Primitive<u8> for usize {
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for usize {
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for usize {
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for usize {
    fn convert(self) -> u64 {
        self as u64
    }
}