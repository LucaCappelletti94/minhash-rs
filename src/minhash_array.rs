//! A fixed-size array of independent MinHash sketches.

use core::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::hasher::Hasher;
use crate::prelude::*;
use crate::primitive::ToU64;

/// An array of `N` independent [`MinHash`] sketches, each with `PERMUTATIONS`
/// words.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHashArray<Word, const PERMUTATIONS: usize, const N: usize, H: Hasher = SipHashes13> {
    #[serde(with = "BigArray")]
    counters: [MinHash<Word, PERMUTATIONS, H>; N],
}
impl<
        Word: Ord + XorShift + Copy + ToU64 + Maximal + PartialEq,
        const PERMUTATIONS: usize,
        const N: usize,
        H: Hasher,
    > PartialEq for MinHashArray<Word, PERMUTATIONS, N, H>
where
    u64: Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        self.counters
            .iter()
            .zip(other.counters.iter())
            .all(|(a, b)| a == b)
    }
}

impl<
        Word: Ord + XorShift + Copy + ToU64 + Maximal + PartialEq,
        const PERMUTATIONS: usize,
        const N: usize,
        H: Hasher,
    > Eq for MinHashArray<Word, PERMUTATIONS, N, H>
where
    u64: Primitive<Word>,
{
}

impl<Word: Maximal, const PERMUTATIONS: usize, const N: usize, H: Hasher> Default
    for MinHashArray<Word, PERMUTATIONS, N, H>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Word: Maximal, const PERMUTATIONS: usize, const N: usize, H: Hasher>
    MinHashArray<Word, PERMUTATIONS, N, H>
{
    /// Creates a new array of empty MinHash sketches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: [MinHash::new(); N],
        }
    }
}

/// We also provide indexing for the MinHashArray.
impl<Word, const PERMUTATIONS: usize, const N: usize, H: Hasher> Index<usize>
    for MinHashArray<Word, PERMUTATIONS, N, H>
{
    type Output = MinHash<Word, PERMUTATIONS, H>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.counters[index]
    }
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H: Hasher> IndexMut<usize>
    for MinHashArray<Word, PERMUTATIONS, N, H>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.counters[index]
    }
}
