//! A fixed-size array of independent MinHash sketches.

use core::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::prelude::*;

/// An array of `N` independent [`MinHash`] sketches, each with `PERMUTATIONS` words.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHashArray<Word, const PERMUTATIONS: usize, const N: usize> {
    #[serde(with = "BigArray")]
    counters: [MinHash<Word, PERMUTATIONS>; N],
}

impl<Word: Maximal, const PERMUTATIONS: usize, const N: usize> Default
    for MinHashArray<Word, PERMUTATIONS, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Word: Maximal, const PERMUTATIONS: usize, const N: usize> MinHashArray<Word, PERMUTATIONS, N> {
    /// Creates a new array of empty MinHash sketches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: [MinHash::new(); N],
        }
    }
}

/// We also provide indexing for the MinHashArray.
impl<W: Maximal, const PERMUTATIONS: usize, const N: usize> Index<usize>
    for MinHashArray<W, PERMUTATIONS, N>
{
    type Output = MinHash<W, PERMUTATIONS>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.counters[index]
    }
}

impl<W: Maximal, const PERMUTATIONS: usize, const N: usize> IndexMut<usize>
    for MinHashArray<W, PERMUTATIONS, N>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.counters[index]
    }
}
