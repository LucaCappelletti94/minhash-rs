//! Union (merge) of MinHash sketches via the bitwise-or operators.

use core::ops::{BitOr, BitOrAssign};

use crate::hasher::Hasher;
use crate::prelude::{Maximal, MinHash, Primitive, XorShift};
use crate::primitive::ToU64;

/// Merge another MinHash into this one, producing the sketch of the union.
///
/// MinHash sketches store, for each permutation, the minimum hash over the
/// elements of a set. Taking the element-wise minimum of two sketches yields
/// the minimum over the union of the two sets, so the bitwise-or operators
/// compute the **union** (merge) of the sketches. There is no way to obtain an
/// intersection sketch by combining two sketches; estimate the Jaccard index
/// instead and derive the intersection cardinality from it.
///
/// When both sketches are in sparse mode, the merge is a cheap sorted-list
/// union. When one or both are dense, the sparse operand(s) are densified
/// first.
impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, H: Hasher, const PERMUTATIONS: usize>
    BitOrAssign<&Self> for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    fn bitor_assign(&mut self, rhs: &Self) {
        self.min_assign(rhs);
    }
}

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, H: Hasher, const PERMUTATIONS: usize>
    BitOrAssign<Self> for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    fn bitor_assign(&mut self, rhs: Self) {
        self.bitor_assign(&rhs);
    }
}

// The `|` operator already signals that the result is meant to be used.
#[allow(clippy::return_self_not_must_use)]
impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, H: Hasher, const PERMUTATIONS: usize>
    BitOr<&Self> for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    type Output = Self;

    fn bitor(self, rhs: &Self) -> Self::Output {
        let mut result = self;
        result.bitor_assign(rhs);
        result
    }
}

#[allow(clippy::return_self_not_must_use)]
impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, H: Hasher, const PERMUTATIONS: usize>
    BitOr<Self> for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result.bitor_assign(&rhs);
        result
    }
}

/// Extension trait adding [`union`](MinHashIterator::union) to iterators of
/// MinHashes.
pub trait MinHashIterator<Word, H, const PERMUTATIONS: usize>
where
    H: Hasher,
{
    /// Merge all MinHashes in the iterator into a single sketch.
    fn union(self) -> MinHash<Word, PERMUTATIONS, H>;
}

impl<
        Word: Maximal + Ord + XorShift + Copy + ToU64,
        H: Hasher,
        const PERMUTATIONS: usize,
        I: Iterator<Item = MinHash<Word, PERMUTATIONS, H>>,
    > MinHashIterator<Word, H, PERMUTATIONS> for I
where
    u64: Primitive<Word>,
{
    fn union(self) -> MinHash<Word, PERMUTATIONS, H> {
        self.fold(MinHash::new(), |mut acc, item| {
            acc |= item;
            acc
        })
    }
}
