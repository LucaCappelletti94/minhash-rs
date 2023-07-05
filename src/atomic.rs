use core::hash::{Hash, Hasher};
use core::{
    mem::transmute,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize},
};

use siphasher::sip128::SipHasher13;

use crate::minhash::MinHash;
use crate::primitive::Primitive;
use crate::splitmix::SplitMix;
use crate::xorshift::XorShift;

pub trait AtomicFetchMin {
    type Word;

    /// Set the minimum value atomically
    ///
    /// # Arguments
    /// * `value` - The value to set.
    /// * `ordering` - The ordering to use.
    ///
    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering);
}

impl AtomicFetchMin for AtomicU8 {
    type Word = u8;

    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU16 {
    type Word = u16;

    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU32 {
    type Word = u32;

    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU64 {
    type Word = u64;

    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicUsize {
    type Word = usize;

    fn set_min(&self, value: Self::Word, ordering: core::sync::atomic::Ordering) {
        self.fetch_min(value, ordering);
    }
}

pub trait AtomicMinHash<Word, AtomicWord: AtomicFetchMin, const PERMUTATIONS: usize>
where
    u64: Primitive<<AtomicWord as AtomicFetchMin>::Word>,
    AtomicWord::Word: XorShift + Copy,
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicWord>
    where
        AtomicWord: 'a,
        Self: 'a;

    /// Insert a value into the MinHash atomically.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    ///
    fn fetch_insert<H: Hash>(&self, value: H, ordering: core::sync::atomic::Ordering) {
        // Create a new hasher.
        let mut hasher = SipHasher13::new();
        // Calculate the hash.
        value.hash(&mut hasher);
        let mut hash: AtomicWord::Word = hasher.finish().splitmix().convert();

        // Iterate over the words.
        for word in self.iter_atomic() {
            // SplitMix the hash.
            hash = hash.xorshift();
            word.set_min(hash, ordering);
        }
    }
}

impl<const PERMUTATIONS: usize> MinHash<u8, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter_atomic(&self) -> impl Iterator<Item = &AtomicU8> {
        let words: &[AtomicU8] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> MinHash<u16, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter_atomic(&self) -> impl Iterator<Item = &AtomicU16> {
        let words: &[AtomicU16] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> MinHash<u32, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter_atomic(&self) -> impl Iterator<Item = &AtomicU32> {
        let words: &[AtomicU32] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> MinHash<u64, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter_atomic(&self) -> impl Iterator<Item = &AtomicU64> {
        let words: &[AtomicU64] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> MinHash<usize, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter_atomic(&self) -> impl Iterator<Item = &AtomicUsize> {
        let words: &[AtomicUsize] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}
