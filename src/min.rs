pub trait Min {
    fn set_min(&mut self, other: Self);

    // Returns true if self is less than other.
    fn is_min(&self, other: Self) -> bool;
}

impl Min for usize {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}

impl Min for u128 {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}

impl Min for u64 {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}

impl Min for u32 {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}

impl Min for u16 {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}

impl Min for u8 {
    fn set_min(&mut self, other: Self) {
        *self = (*self).min(other);
    }

    fn is_min(&self, other: Self) -> bool {
        *self <= other
    }
}
