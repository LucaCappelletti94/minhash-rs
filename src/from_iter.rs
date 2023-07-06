use std::hash::Hash;

use crate::{prelude::{Maximal, Min, MinHash, Primitive, XorShift}, zero::Zero};

impl<Word: Min + Clone + Eq + Maximal + XorShift + Zero, A: Hash, const PERMUTATATIONS: usize>
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
    /// let minhash = MinHash::<u64, 128>::from_iter(data);
    /// 
    /// ```
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut hll = Self::new();
        for item in iter {
            hll.insert(item);
        }
        hll
    }
}
