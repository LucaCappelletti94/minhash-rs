//! `FromIterator` implementation that builds a MinHash from an iterator.

use core::hash::Hash as CoreHash;

use crate::batched;
use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

impl<Word, const PERMUTATIONS: usize, H, Hash, Value> core::iter::FromIterator<Value>
    for MinHash<Word, PERMUTATIONS, H, Hash, Value>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Value: CoreHash,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    /// Build a dense MinHash from an iterator via the batched loop-swap
    /// path.
    ///
    /// Digests are hashed and prefaced by two `splitmix` rounds into a
    /// bounded stack buffer, then, for each permutation slot, every
    /// buffered digest is advanced one xorshift step and the minimum is
    /// reduced into that slot. The result is bit-identical to iterating
    /// [`insert`](MinHash::insert) over the same input, at roughly a
    /// third of the wall time for the default configuration.
    ///
    /// For a sparse prefix, seed a
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
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        let mut sig = Self::new();
        batched::build_into::<Word, PERMUTATIONS, H, Hash, Value, _>(sig.as_words_mut(), iter);
        sig
    }
}
