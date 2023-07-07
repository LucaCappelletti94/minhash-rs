//! Module providing the MinHash data structure.
//!
//!

use crate::{
    prelude::{Min, Primitive},
    splitmix::SplitMix,
    xorshift::XorShift,
    zero::Zero,
};
use core::hash::{Hash, Hasher};
use core::ops::Index;
use core::ops::IndexMut;
use siphasher::sip::SipHasher13;

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
    /// minhash.insert(42);
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
    ///    minhash.insert(i);
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
    u64: Primitive<Word>,
{
    /// Returns whether the MinHash may contain the provided value.
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
    /// assert!(!minhash.may_contain_value(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain_value(42));
    /// minhash.insert(47);
    /// assert!(minhash.may_contain_value(47));
    /// ```
    ///
    pub fn may_contain_value<H: Hash>(&self, value: H) -> bool {
        self.iter()
            .zip(Self::iter_hashes_from_value(value))
            .all(|(word, hash)| word.is_min(hash))
    }

    /// Insert a value into the MinHash.
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
    /// assert!(!minhash.may_contain_value(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain_value(42));
    /// minhash.insert(47);
    /// assert!(minhash.may_contain_value(47));
    /// ```
    pub fn insert<H: Hash>(&mut self, value: H) {
        for (word, hash) in self.iter_mut().zip(Self::iter_hashes_from_value(value)) {
            word.set_min(hash);
        }
    }

    /// Iterate on the hashes from the provided value.
    pub fn iter_hashes_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        // Create a new hasher.
        let mut hasher = SipHasher13::new();
        // Calculate the hash.
        value.hash(&mut hasher);
        let mut hash: Word = hasher.finish().splitmix().convert();

        // Iterate over the words.
        (0..PERMUTATIONS).map(move |_| {
            hash = hash.xorshift();
            hash
        })
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
