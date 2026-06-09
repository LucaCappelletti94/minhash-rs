//! Union (merge) of MinHash sketches via the bitwise-or operators.

use std::ops::{BitOr, BitOrAssign};

use crate::prelude::{Maximal, Min, MinHash};

/// Merge another MinHash into this one, producing the sketch of the union.
///
/// MinHash sketches store, for each permutation, the minimum hash over the
/// elements of a set. Taking the element-wise minimum of two sketches yields
/// the minimum over the union of the two sets, so the bitwise-or operators
/// compute the **union** (merge) of the sketches. There is no way to obtain an
/// intersection sketch by combining two sketches; estimate the Jaccard index
/// instead and derive the intersection cardinality from it.
impl<Word: Min + Clone + Eq, const PERMUTATIONS: usize> BitOrAssign<&Self>
    for MinHash<Word, PERMUTATIONS>
{
    fn bitor_assign(&mut self, rhs: &Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(left, right)| {
            left.set_min(right.clone());
        });
    }
}

impl<Word: Min + Clone + Eq, const PERMUTATIONS: usize> BitOrAssign<Self>
    for MinHash<Word, PERMUTATIONS>
{
    fn bitor_assign(&mut self, rhs: Self) {
        self.bitor_assign(&rhs);
    }
}

// The `|` operator already signals that the result is meant to be used.
#[allow(clippy::return_self_not_must_use)]
impl<Word: Min + Clone + Eq, const PERMUTATIONS: usize> BitOr for MinHash<Word, PERMUTATIONS> {
    type Output = Self;

    /// Returns the union (merge) of two MinHash sketches.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashSet;
    /// use minhash_rs::prelude::*;
    ///
    /// let a: HashSet<u64> = (1..=8).collect();
    /// let b: HashSet<u64> = (5..=12).collect();
    /// let union: HashSet<u64> = a.union(&b).copied().collect();
    ///
    /// let mh_a: MinHash<u64, 256> = a.iter().collect();
    /// let mh_b: MinHash<u64, 256> = b.iter().collect();
    /// let mh_union: MinHash<u64, 256> = union.iter().collect();
    ///
    /// // Merging two sketches yields the sketch of the union of the two sets.
    /// assert_eq!(mh_a | mh_b, mh_union);
    /// ```
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.bitor_assign(rhs);
        self
    }
}

#[allow(clippy::return_self_not_must_use)]
impl<Word: Min + Clone + Eq, const PERMUTATIONS: usize> BitOr<&Self>
    for MinHash<Word, PERMUTATIONS>
{
    type Output = Self;

    fn bitor(mut self, rhs: &Self) -> Self::Output {
        self.bitor_assign(rhs);
        self
    }
}

/// Extension trait adding [`union`](MinHashIterator::union) to iterators of MinHashes.
pub trait MinHashIterator<Word: Min + Eq, const PERMUTATIONS: usize> {
    /// Returns a MinHash that is the union (merge) of all MinHashes in the iterator.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashSet;
    /// use minhash_rs::prelude::*;
    ///
    /// let a: HashSet<u64> = (1..=8).collect();
    /// let b: HashSet<u64> = (5..=12).collect();
    /// let c: HashSet<u64> = (20..=30).collect();
    /// let all: HashSet<u64> = a.union(&b).copied().chain(c.iter().copied()).collect();
    ///
    /// let sketches = vec![
    ///     a.iter().collect::<MinHash<u64, 256>>(),
    ///     b.iter().collect::<MinHash<u64, 256>>(),
    ///     c.iter().collect::<MinHash<u64, 256>>(),
    /// ];
    ///
    /// let merged = sketches.into_iter().union();
    /// let expected: MinHash<u64, 256> = all.iter().collect();
    ///
    /// assert_eq!(merged, expected);
    /// ```
    fn union(self) -> MinHash<Word, PERMUTATIONS>;
}

impl<
        Word: Maximal + Min + Eq,
        const PERMUTATIONS: usize,
        I: Iterator<Item = MinHash<Word, PERMUTATIONS>>,
    > MinHashIterator<Word, PERMUTATIONS> for I
{
    fn union(self) -> MinHash<Word, PERMUTATIONS> {
        let mut result: MinHash<Word, PERMUTATIONS> = MinHash::default();
        for minhash in self {
            result |= minhash;
        }
        result
    }
}
