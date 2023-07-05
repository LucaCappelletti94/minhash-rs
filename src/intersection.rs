use std::ops::{BitAnd, BitAndAssign};

use crate::prelude::{Maximal, Min, MinHash};

impl<Word: Min + Clone + Eq, const PERMUTATATIONS: usize> BitAndAssign<&Self>
    for MinHash<Word, PERMUTATATIONS>
{
    fn bitand_assign(&mut self, rhs: &Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(left, right)| {
            left.set_min(right.clone());
        });
    }
}

impl<Word: Min + Clone + Eq, const PERMUTATATIONS: usize> BitAndAssign<Self>
    for MinHash<Word, PERMUTATATIONS>
{
    fn bitand_assign(&mut self, rhs: Self) {
        self.bitand_assign(&rhs);
    }
}

impl<Word: Min + Clone + Eq, const PERMUTATATIONS: usize> BitAnd for MinHash<Word, PERMUTATATIONS> {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self.bitand_assign(rhs);
        self
    }
}

impl<Word: Min + Clone + Eq, const PERMUTATATIONS: usize> BitAnd<&Self>
    for MinHash<Word, PERMUTATATIONS>
{
    type Output = Self;

    fn bitand(mut self, rhs: &Self) -> Self::Output {
        self.bitand_assign(rhs);
        self
    }
}

pub trait MinHashIterator<Word: Min + Eq, const PERMUTATIONS: usize> {
    /// Returns a MinHash that is the intersection of all MinHashes in the iterator.
    ///
    /// # Example
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    /// ```
    fn intersection(self) -> MinHash<Word, PERMUTATIONS>;
}

impl<
        Word: Maximal + Min + Eq,
        const PERMUTATIONS: usize,
        I: Iterator<Item = MinHash<Word, PERMUTATIONS>>,
    > MinHashIterator<Word, PERMUTATIONS> for I
{
    fn intersection(self) -> MinHash<Word, PERMUTATIONS> {
        let mut result: MinHash<Word, PERMUTATIONS> = MinHash::default();
        for minhash in self {
            result &= minhash;
        }
        result
    }
}
