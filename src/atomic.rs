use core::hash::{Hash, Hasher};
use core::{
    mem::transmute,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize},
};

use fnv::FnvHasher;
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
    /// Iterate on the hashes from the provided value and hasher.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    fn iter_hashes_from_value<H: Hash, HS: Hasher>(
        value: H,
        mut hasher: HS,
    ) -> impl Iterator<Item = Word> {
        // Calculate the hash.
        value.hash(&mut hasher);
        let mut hash: Word = hasher.finish().splitmix().splitmix().convert();

        // Iterate over the words.
        (0..PERMUTATIONS).map(move |_| {
            hash = hash.xorshift();
            hash
        })
    }

    /// Iterate on the SipHasher13 hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    ///
    /// assert!(!minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(42);
    /// assert!(minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(47);
    /// assert!(minhash.may_contain_value_with_siphashes13(47));
    /// ```
    ///  
    fn iter_siphashes13_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, SipHasher13::new())
    }

    /// Iterate on the keyed SipHasher13 hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key0 = 0x0123456789ABCDEF;
    /// let key1 = 0xFEDCBA9876543210;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(42, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(47, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(47, key0, key1));
    /// ```
    ///
    fn iter_keyed_siphashes13_from_value<H: Hash>(
        value: H,
        key0: u64,
        key1: u64,
    ) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, SipHasher13::new_with_keys(key0, key1))
    }

    /// Iterate on the FVN hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    ///
    /// assert!(!minhash.may_contain_value_with_fvn(42));
    /// minhash.insert_with_fvn(42);
    /// assert!(minhash.may_contain_value_with_fvn(42));
    /// minhash.insert_with_fvn(47);
    /// assert!(minhash.may_contain_value_with_fvn(47));
    /// ```
    ///  
    fn iter_fvn_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, FnvHasher::default())
    }

    /// Iterate on the keyed SipHasher13 hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    /// * `key` - The first key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fvn(42, key));
    ///
    fn iter_keyed_fvn_from_value<H: Hash>(value: H, key: u64) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, FnvHasher::with_key(key))
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

    /// Insert a value into the MinHash atomically, with SipHasher13.
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
    /// assert!(!minhash.may_contain_value_with_siphashes13(42));
    /// minhash.fetch_insert_with_siphashes13(42, core::sync::atomic::Ordering::Relaxed);
    /// assert!(!minhash.is_empty());
    /// assert!(
    ///     minhash.may_contain_value_with_siphashes13(42),
    ///     concat!(
    ///         "The MinHash should contain the value 42, ",
    ///         "but it does not. The MinHash is: {:?}. ",
    ///         "The hashes associated to the value 42 are: {:?}."
    ///     ),
    ///     minhash,
    ///     MinHash::<u64, 4>::iter_siphashes13_from_value(42).collect::<Vec<_>>()
    /// );
    /// minhash.fetch_insert_with_siphashes13(47, core::sync::atomic::Ordering::Relaxed);
    /// assert!(minhash.may_contain_value_with_siphashes13(47));
    ///
    /// ```
    ///
    fn fetch_insert_with_siphashes13<H: Hash>(
        &self,
        value: H,
        ordering: core::sync::atomic::Ordering,
    ) {
        // Iterate over the words.
        for (word, hash) in self
            .iter_atomic()
            .zip(Self::iter_siphashes13_from_value(value))
        {
            word.set_min(hash, ordering);
        }
    }

    /// Insert a value into the MinHash atomically, with keyed SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 4>::new();
    /// let key0 = 0x0123456789ABCDEF;
    /// let key1 = 0xFEDCBA9876543210;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.fetch_insert_with_keyed_siphashes13(42, key0, key1, core::sync::atomic::Ordering::Relaxed);
    /// assert!(!minhash.is_empty());
    /// assert!(
    ///     minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1),
    ///     concat!(
    ///         "The MinHash should contain the value 42, ",
    ///         "but it does not. The MinHash is: {:?}. ",
    ///         "The hashes associated to the value 42 are: {:?}."
    ///     ),
    ///     minhash,
    ///     MinHash::<u64, 4>::iter_keyed_siphashes13_from_value(42, key0, key1).collect::<Vec<_>>()
    /// );
    /// minhash.fetch_insert_with_keyed_siphashes13(47, key0, key1, core::sync::atomic::Ordering::Relaxed);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(47, key0, key1));
    ///
    /// ```
    ///
    fn fetch_insert_with_keyed_siphashes13<H: Hash>(
        &self,
        value: H,
        key0: u64,
        key1: u64,
        ordering: core::sync::atomic::Ordering,
    ) {
        // Iterate over the words.
        for (word, hash) in self
            .iter_atomic()
            .zip(Self::iter_keyed_siphashes13_from_value(value, key0, key1))
        {
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
