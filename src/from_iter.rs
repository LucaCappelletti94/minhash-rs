use std::hash::Hash;

use crate::prelude::{Maximal, Min, MinHash, Primitive, XorShift};

impl<Word: Min + Clone + Eq + Maximal + XorShift, A: Hash, const PERMUTATATIONS: usize>
    core::iter::FromIterator<A> for MinHash<Word, PERMUTATATIONS>
where
    u64: Primitive<Word>,
{
    #[inline(always)]
    /// Creates a new MinHash and adds all elements from an iterator to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    /// let minhash = MinHash::<u64, 128>::from_iter(data.clone());
    ///
    /// for item in data {
    ///     assert!(minhash.may_contain_value_with_siphashes13(item));
    /// }
    ///
    /// ```
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut hll = Self::new();
        for item in iter {
            hll.insert_with_siphashes13(item);
        }
        hll
    }
}
