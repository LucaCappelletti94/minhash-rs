use core::hash::{Hash, Hasher};
use core::{
    mem::transmute,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize},
};

use siphasher::sip128::SipHasher13;

use crate::prelude::*;

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

pub trait IterHashes<Word, const PERMUTATIONS: usize>
where
    Word: Min + XorShift + Copy + Eq,
    u64: Primitive<Word>,
{
    /// Iterate on the hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    fn iter_hashes_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        // Create a new hasher.
        let mut hasher = SipHasher13::new();
        // Calculate the hash.
        value.hash(&mut hasher);
        let mut hash: Word = hasher.finish().splitmix().splitmix().convert();

        // Iterate over the words.
        (0..PERMUTATIONS).map(move |_| {
            hash = hash.xorshift();
            hash
        })
    }
}

impl<Word: Min + XorShift + Copy + Eq, const PERMUTATIONS: usize> IterHashes<Word, PERMUTATIONS>
    for MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
}

pub trait AtomicMinHash<AtomicWord: AtomicFetchMin, const PERMUTATIONS: usize>
where
    Self: IterHashes<AtomicWord::Word, PERMUTATIONS>,
    AtomicWord::Word: Min + XorShift + Copy + Eq,
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
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 4>::new();
    ///
    /// assert!(!minhash.may_contain_value(42));
    /// minhash.fetch_insert(42, core::sync::atomic::Ordering::Relaxed);
    /// assert!(!minhash.is_empty());
    /// assert!(
    ///     minhash.may_contain_value(42),
    ///     concat!(
    ///         "The MinHash should contain the value 42, ",
    ///         "but it does not. The MinHash is: {:?}. ",
    ///         "The hashes associated to the value 42 are: {:?}."
    ///     ),
    ///     minhash,
    ///     MinHash::<u64, 4>::iter_hashes_from_value(42).collect::<Vec<_>>()
    /// );
    /// minhash.fetch_insert(47, core::sync::atomic::Ordering::Relaxed);
    /// assert!(minhash.may_contain_value(47));
    ///
    /// ```
    ///
    fn fetch_insert<H: Hash>(&self, value: H, ordering: core::sync::atomic::Ordering) {
        // Iterate over the words.
        for (word, hash) in self.iter_atomic().zip(Self::iter_hashes_from_value(value)) {
            word.set_min(hash, ordering);
        }
    }
}

impl<const PERMUTATIONS: usize> AtomicMinHash<AtomicU8, PERMUTATIONS>
    for MinHash<u8, PERMUTATIONS>
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicU8>
    where
        Self: 'a,
    {
        let words: &[AtomicU8] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> AtomicMinHash<AtomicU16, PERMUTATIONS>
    for MinHash<u16, PERMUTATIONS>
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicU16>
    where
        Self: 'a,
    {
        let words: &[AtomicU16] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> AtomicMinHash<AtomicU32, PERMUTATIONS>
    for MinHash<u32, PERMUTATIONS>
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicU32>
    where
        Self: 'a,
    {
        let words: &[AtomicU32] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> AtomicMinHash<AtomicU64, PERMUTATIONS>
    for MinHash<u64, PERMUTATIONS>
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicU64>
    where
        Self: 'a,
    {
        let words: &[AtomicU64] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}

impl<const PERMUTATIONS: usize> AtomicMinHash<AtomicUsize, PERMUTATIONS>
    for MinHash<usize, PERMUTATIONS>
{
    /// Iterate over the words.
    fn iter_atomic<'a>(&'a self) -> impl Iterator<Item = &'a AtomicUsize>
    where
        Self: 'a,
    {
        let words: &[AtomicUsize] = unsafe { transmute(self.as_ref()) };
        words.iter()
    }
}
