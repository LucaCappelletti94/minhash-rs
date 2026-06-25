//! LSH banding for MinHash signatures.

use crate::prelude::MinHash;
use core::hash::{Hash, Hasher};
use fnv::FnvHasher;

/// FNV-1a hash of a band of MinHash registers.
///
/// ```
/// use minhash_rs::lsh::band_hash;
///
/// let h = band_hash(&[1u64, 2, 3]);
/// let same = band_hash(&[1u64, 2, 3]);
/// assert_eq!(h, same);
/// ```
pub fn band_hash<Word: Hash>(band: &[Word]) -> u64 {
    let mut hasher = FnvHasher::default();
    for register in band {
        register.hash(&mut hasher);
    }
    hasher.finish()
}

/// Iterator over band indices where two sets of band hashes are equal.
#[derive(Debug, Clone)]
pub struct BandMatches<'a, const BANDS: usize> {
    a: &'a [u64; BANDS],
    b: &'a [u64; BANDS],
    idx: usize,
}

impl<'a, const BANDS: usize> BandMatches<'a, BANDS> {
    /// Create from two band hash arrays.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let a = band_hash(&[1u64]);
    /// let b = band_hash(&[2u64]);
    /// let c = band_hash(&[1u64]); // same as `a`
    /// let left = [a, b];
    /// let right = [c, b];
    /// let mut matches = BandMatches::new(&left, &right); // only band 0 matches
    /// assert_eq!(matches.next(), Some(0));
    /// ```
    #[must_use]
    pub fn new(a: &'a [u64; BANDS], b: &'a [u64; BANDS]) -> Self {
        BandMatches { a, b, idx: 0 }
    }
}

impl<const BANDS: usize> Iterator for BandMatches<'_, BANDS> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while self.idx < BANDS {
            let i = self.idx;
            self.idx += 1;
            if self.a[i] == self.b[i] {
                return Some(i);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(BANDS - self.idx))
    }
}

impl<Word: Hash, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Band hashes of this signature: `BANDS` consecutive bands of `PERMUTATIONS / BANDS` registers.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let sketch: MinHash<u64, 128> = (0..100u64).collect();
    /// let hashes = sketch.band_hashes::<16>();
    /// assert_eq!(hashes.len(), 16);
    /// ```
    #[must_use]
    pub fn band_hashes<const BANDS: usize>(&self) -> [u64; BANDS] {
        let registers = self.as_ref();
        core::array::from_fn(|band| {
            let rows = PERMUTATIONS / BANDS;
            band_hash(&registers[band * rows..(band + 1) * rows])
        })
    }
}
