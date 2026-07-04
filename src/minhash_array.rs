//! A fixed-size array of independent MinHash sketches.

use core::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::hasher::{Hasher, SipHashes13};
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

/// An array of `N` independent [`MinHash`] sketches, each with `PERMUTATIONS`
/// permutations. Handy when you want a small collection of MinHashes with
/// identical parameters (for example, one per shard or per timestep) without
/// spelling the type parameters at every element.
#[repr(transparent)]
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHashArray<
    Word,
    const PERMUTATIONS: usize,
    const N: usize,
    H: Hasher = SipHashes13,
    Hash: HashType = u64,
> where
    Hash: Primitive<Word>,
{
    #[serde(with = "BigArray")]
    counters: [MinHash<Word, PERMUTATIONS, H, Hash>; N],
}

impl<
        Word: core::fmt::Debug,
        const PERMUTATIONS: usize,
        const N: usize,
        H: Hasher,
        Hash: HashType,
    > core::fmt::Debug for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MinHashArray")
            .field("counters", &self.counters)
            .finish()
    }
}

impl<Word: Clone, const PERMUTATIONS: usize, const N: usize, H: Hasher, Hash: HashType> Clone
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn clone(&self) -> Self {
        Self {
            counters: self.counters.clone(),
        }
    }
}

impl<Word: Copy, const PERMUTATIONS: usize, const N: usize, H: Hasher, Hash: HashType> Copy
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Hash: Primitive<Word>,
{
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash> PartialEq
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Word: Ord + Copy + Maximal + PartialEq + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        self.counters
            .iter()
            .zip(other.counters.iter())
            .all(|(a, b)| a == b)
    }
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash> Eq
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Word: Ord + Copy + Maximal + PartialEq + Primitive<Hash>,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash> Default
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Word: Maximal,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash>
    MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    Word: Maximal,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    /// Create an array of `N` empty MinHash sketches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: [MinHash::new(); N],
        }
    }
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash> Index<usize>
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    type Output = MinHash<Word, PERMUTATIONS, H, Hash>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.counters[index]
    }
}

impl<Word, const PERMUTATIONS: usize, const N: usize, H, Hash> IndexMut<usize>
    for MinHashArray<Word, PERMUTATIONS, N, H, Hash>
where
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.counters[index]
    }
}
