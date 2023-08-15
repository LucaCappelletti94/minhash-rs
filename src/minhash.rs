//! Module providing the MinHash data structure.

use crate::{
    atomic::IterHashes,
    prelude::{Min, Primitive},
    xorshift::XorShift,
    zero::Zero,
};
use core::hash::Hash;
use core::ops::Index;
use core::ops::IndexMut;

use crate::prelude::Maximal;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MinHash<Word, const PERMUTATIONS: usize> {
    words: [Word; PERMUTATIONS],
}

impl<Word: Maximal, const PERMUTATIONS: usize> Default for MinHash<Word, PERMUTATIONS> {
    /// Create a new MinHash with the maximal value.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::default();
    ///
    /// assert_eq!(minhash, MinHash::<u64, 128>::new());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl<Word: Maximal, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Create a new MinHash.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            words: [Word::maximal(); PERMUTATIONS],
        }
    }
}

impl<Word: Min + XorShift + Copy + Eq + Maximal + Zero, const PERMUTATIONS: usize>
    MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
    /// Returns whether the MinHash is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    ///
    /// assert!(minhash.is_empty());
    /// minhash.insert_with_siphashes13(42);
    /// assert!(!minhash.is_empty());
    /// ```
    ///
    pub fn is_empty(&self) -> bool {
        self.iter().all(|word| *word == Word::maximal())
    }

    /// Returns whether the MinHash is fully saturated.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    ///
    /// assert!(!minhash.is_full());
    ///
    /// for i in 0..1024 {
    ///    minhash.insert_with_siphashes13(i);
    /// }
    ///
    /// assert!(minhash.is_full());
    /// ```
    ///
    pub fn is_full(&self) -> bool {
        self.iter().all(|word| *word == Word::zero())
    }
}

impl<Word: Min + XorShift + Copy + Eq, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS>
where
    Self: IterHashes<Word, PERMUTATIONS>,
    u64: Primitive<Word>,
{
    /// Returns whether the MinHash may contain the provided value, using the SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    ///
    /// # Implementative details
    /// The procedure estimates whether the provided value is contained
    /// in the current MinHash data structure by checking whether all of
    /// the words are smaller or equal to all of the hash values that
    /// are calculated using the provided value as seed.
    ///
    /// # Examples
    ///
    /// ```
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
    pub fn may_contain_value_with_siphashes13<H: Hash>(&self, value: H) -> bool {
        self.iter()
            .zip(Self::iter_siphashes13_from_value(value))
            .all(|(word, hash)| word.is_min(hash))
    }

    /// Insert a value into the MinHash using the SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    ///
    /// # Examples
    /// In the following example we show how we can
    /// create a MinHash and insert a value in it.
    ///
    /// ```
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
    pub fn insert_with_siphashes13<H: Hash>(&mut self, value: H) {
        for (word, hash) in self
            .iter_mut()
            .zip(Self::iter_siphashes13_from_value(value))
        {
            word.set_min(hash);
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the keyed SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Implementative details
    /// The procedure estimates whether the provided value is contained
    /// in the current MinHash data structure by checking whether all of
    /// the words are smaller or equal to all of the hash values that
    /// are calculated using the provided value as seed.
    ///
    /// # Examples
    ///
    /// ```
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
    pub fn may_contain_value_with_keyed_siphashes13<H: Hash>(
        &self,
        value: H,
        key0: u64,
        key1: u64,
    ) -> bool {
        self.iter()
            .zip(Self::iter_keyed_siphashes13_from_value(value, key0, key1))
            .all(|(word, hash)| word.is_min(hash))
    }

    /// Insert a value into the MinHash using the keyed SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    /// In the following example we show how we can
    /// create a MinHash and insert a value in it.
    ///
    /// ```
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
    pub fn insert_with_keyed_siphashes13<H: Hash>(&mut self, value: H, key0: u64, key1: u64) {
        for (word, hash) in self
            .iter_mut()
            .zip(Self::iter_keyed_siphashes13_from_value(value, key0, key1))
        {
            word.set_min(hash);
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the FVN.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    ///
    /// # Implementative details
    /// The procedure estimates whether the provided value is contained
    /// in the current MinHash data structure by checking whether all of
    /// the words are smaller or equal to all of the hash values that
    /// are calculated using the provided value as seed.
    ///
    /// # Examples
    ///
    /// ```
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
    pub fn may_contain_value_with_fvn<H: Hash>(&self, value: H) -> bool {
        self.iter()
            .zip(Self::iter_fvn_from_value(value))
            .all(|(word, hash)| word.is_min(hash))
    }

    /// Insert a value into the MinHash using the FVN.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    ///
    /// # Examples
    /// In the following example we show how we can
    /// create a MinHash and insert a value in it.
    ///
    /// ```
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
    pub fn insert_with_fvn<H: Hash>(&mut self, value: H) {
        for (word, hash) in self.iter_mut().zip(Self::iter_fvn_from_value(value)) {
            word.set_min(hash);
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the keyed FVN.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    /// * `key` - The first key.
    ///
    /// # Implementative details
    /// The procedure estimates whether the provided value is contained
    /// in the current MinHash data structure by checking whether all of
    /// the words are smaller or equal to all of the hash values that
    /// are calculated using the provided value as seed.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fvn(42, key));
    /// minhash.insert_with_keyed_fvn(42, key);
    /// assert!(minhash.may_contain_value_with_keyed_fvn(42, key));
    /// minhash.insert_with_keyed_fvn(47, key);
    /// assert!(minhash.may_contain_value_with_keyed_fvn(47, key));
    /// ```
    ///
    pub fn may_contain_value_with_keyed_fvn<H: Hash>(&self, value: H, key: u64) -> bool {
        self.iter()
            .zip(Self::iter_keyed_fvn_from_value(value, key))
            .all(|(word, hash)| word.is_min(hash))
    }

    /// Insert a value into the MinHash using the keyed FVN.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    /// * `key` - The first key.
    ///
    /// # Examples
    /// In the following example we show how we can
    /// create a MinHash and insert a value in it.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fvn(42, key));
    /// minhash.insert_with_keyed_fvn(42, key);
    /// assert!(minhash.may_contain_value_with_keyed_fvn(42, key));
    /// minhash.insert_with_keyed_fvn(47, key);
    /// assert!(minhash.may_contain_value_with_keyed_fvn(47, key));
    /// ```
    pub fn insert_with_keyed_fvn<H: Hash>(&mut self, value: H, key: u64) {
        for (word, hash) in self
            .iter_mut()
            .zip(Self::iter_keyed_fvn_from_value(value, key))
        {
            word.set_min(hash);
        }
    }
}

impl<Word, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Iterate over the words.
    pub fn iter(&self) -> impl Iterator<Item = &Word> {
        self.words.iter()
    }

    /// Iterate over the words mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Word> {
        self.words.iter_mut()
    }

    /// Returns the number of permutations.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    ///
    /// assert_eq!(minhash.number_of_permutations(), 128);
    /// ```
    pub fn number_of_permutations(&self) -> usize {
        PERMUTATIONS
    }

    /// Returns memory required to store the MinHash in bits.
    ///
    /// # Examples
    /// For a MinHash with 128 permutations and 64 bit words, the memory required is 128 * 64 * 8.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    ///
    /// assert_eq!(minhash.memory(), 128 * 64);
    /// ```
    ///
    /// For a MinHash with 128 permutations and 32 bit words, the memory required is 128 * 32 * 8.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u32, 128>::new();
    ///
    /// assert_eq!(minhash.memory(), 128 * 32);
    /// ```
    ///
    pub fn memory(&self) -> usize {
        PERMUTATIONS * core::mem::size_of::<Word>() * 8
    }
}

impl<Word: Eq, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Calculate the similarity between two MinHashes.
    ///
    /// # Arguments
    /// * `other` - The other MinHash to compare to.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashSet;
    /// use minhash_rs::prelude::*;
    ///
    /// let first_set: HashSet<u64> = [1_u64, 2_u64, 3_u64, 4_u64, 5_u64, 6_u64, 7_u64, 8_u64].iter().copied().collect();
    /// let second_set: HashSet<u64> = [5_u64, 6_u64, 7_u64, 8_u64, 9_u64, 10_u64, 11_u64, 12_u64].iter().copied().collect();
    ///
    /// let mut first_minhash: MinHash<u64, 128> = first_set.iter().collect();
    /// let mut second_minhash: MinHash<u64, 128> = second_set.iter().collect();
    ///
    /// let approximation = first_minhash.estimate_jaccard_index(&second_minhash);
    /// let ground_truth = first_set.intersection(&second_set).count() as f64 / first_set.union(&second_set).count() as f64;
    ///
    /// assert!((approximation - ground_truth).abs() < 0.01, concat!(
    ///     "We expected the approximation to be close to the ground truth, ",
    ///    "but got an error of {} instead. The ground truth is {} and the approximation is {}."
    ///    ), (approximation - ground_truth).abs(), ground_truth, approximation
    /// );
    /// ```
    pub fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        self.iter()
            .zip(other.iter())
            .map(|(l, r)| (l == r) as usize)
            .sum::<usize>() as f64
            / PERMUTATIONS as f64
    }
}

/// We also implement AsRef and AsMut for direct access on the MinHash words.
impl<Word, const PERMUTATIONS: usize> AsRef<[Word]> for MinHash<Word, PERMUTATIONS> {
    fn as_ref(&self) -> &[Word] {
        &self.words
    }
}

impl<Word, const PERMUTATIONS: usize> AsMut<[Word]> for MinHash<Word, PERMUTATIONS> {
    fn as_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }
}

/// We also provide indexing for the MinHash.
impl<W: Maximal, const PERMUTATIONS: usize> Index<usize> for MinHash<W, PERMUTATIONS> {
    type Output = W;

    fn index(&self, index: usize) -> &Self::Output {
        &self.words[index]
    }
}

impl<W: Maximal, const PERMUTATIONS: usize> IndexMut<usize> for MinHash<W, PERMUTATIONS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.words[index]
    }
}
