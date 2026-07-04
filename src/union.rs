//! Union (merge) of MinHash sketches via the bitwise-or operators.

use core::ops::{BitOr, BitOrAssign};

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

// MinHash stores per-permutation minimums; taking the element-wise min of
// two sketches yields the sketch of the union of the underlying sets. This is
// therefore a union (merge), not an intersection. Sparse-vs-sparse operands
// use the cheap sorted-list merge; mixed modes densify the sparse operand
// first.

impl<Word, H, const PERMUTATIONS: usize, Hash> BitOrAssign<&Self>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    fn bitor_assign(&mut self, rhs: &Self) {
        self.min_assign(rhs);
    }
}

impl<Word, H, const PERMUTATIONS: usize, Hash> BitOrAssign<Self>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    fn bitor_assign(&mut self, rhs: Self) {
        self.bitor_assign(&rhs);
    }
}

#[allow(clippy::return_self_not_must_use)]
impl<Word, H, const PERMUTATIONS: usize, Hash> BitOr<&Self> for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    type Output = Self;

    fn bitor(self, rhs: &Self) -> Self::Output {
        let mut result = self;
        result.bitor_assign(rhs);
        result
    }
}

#[allow(clippy::return_self_not_must_use)]
impl<Word, H, const PERMUTATIONS: usize, Hash> BitOr<Self> for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result.bitor_assign(&rhs);
        result
    }
}

/// Adds [`union`](MinHashIterator::union) to iterators of MinHashes.
pub trait MinHashIterator<Word, H, const PERMUTATIONS: usize, Hash>
where
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    /// Merge every sketch in the iterator into a single union.
    fn union(self) -> MinHash<Word, PERMUTATIONS, H, Hash>;
}

impl<Word, H, const PERMUTATIONS: usize, Hash, I> MinHashIterator<Word, H, PERMUTATIONS, Hash> for I
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
    I: Iterator<Item = MinHash<Word, PERMUTATIONS, H, Hash>>,
{
    fn union(self) -> MinHash<Word, PERMUTATIONS, H, Hash> {
        self.fold(MinHash::new(), |mut acc, item| {
            acc |= item;
            acc
        })
    }
}
