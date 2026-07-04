//! `FromIterator` implementation that builds a MinHash from an iterator.

use core::hash::Hash as CoreHash;

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

impl<Word, A, H, const PERMUTATIONS: usize, Hash> core::iter::FromIterator<A>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    A: CoreHash,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    /// Build a dense MinHash from an iterator, inserting each element in
    /// turn. For a sparse prefix, seed a
    /// [`SparseHashes`](crate::sparse_hashes::SparseHashes) or
    /// [`SparseValues`](crate::sparse_values::SparseValues) directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let minhash: MinHash<u64, 128> = data.iter().copied().collect();
    /// for item in data {
    ///     assert!(minhash.may_contain(item));
    /// }
    /// ```
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut minhash = Self::new();
        for item in iter {
            minhash.insert(item);
        }
        minhash
    }
}
