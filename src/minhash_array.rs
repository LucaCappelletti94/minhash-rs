use core::ops::{Index, IndexMut};

use crate::prelude::*;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinHashArray<Word, const PERMUTATIONS: usize, const N: usize> {
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
