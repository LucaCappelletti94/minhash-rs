//! LSH banding on MinHash signatures.

use core::hash::{Hash as CoreHash, Hasher};

use fnv::FnvHasher;

/// FNV-1a hash of a band of MinHash registers.
///
/// The `LSH` (locality-sensitive hashing) banding technique breaks a signature
/// into equal-sized bands and hashes each band into a single fingerprint;
/// candidate pairs are pairs of signatures whose fingerprint matches in at
/// least one band.
///
/// ```
/// use minhash_rs::lsh::band_hash;
///
/// let h = band_hash(&[1u64, 2, 3]);
/// let same = band_hash(&[1u64, 2, 3]);
/// assert_eq!(h, same);
/// ```
pub fn band_hash<Word: CoreHash>(band: &[Word]) -> u64 {
    let mut hasher = FnvHasher::default();
    for register in band {
        register.hash(&mut hasher);
    }
    hasher.finish()
}

/// Split a dense signature into `BANDS` bands and FNV-1a hash each.
///
/// Extracted as a free helper so the [`crate::min_hasher::MinHasher`] trait
/// default `band_hashes` can call it without recursing through the trait
/// method itself.
pub(crate) fn dense_band_hashes<Word: CoreHash, const P: usize, const BANDS: usize>(
    words: &[Word; P],
) -> [u64; BANDS] {
    const {
        assert!(
            BANDS >= 1 && P % BANDS == 0,
            "band_hashes: BANDS must be at least 1 and must evenly divide PERMUTATIONS",
        );
    }
    let rows = P / BANDS;
    core::array::from_fn(|band| band_hash(&words[band * rows..(band + 1) * rows]))
}

/// Iterator over band indices where two arrays of band hashes agree.
#[derive(Debug, Clone)]
pub struct BandMatches<'a, const BANDS: usize> {
    a: &'a [u64; BANDS],
    b: &'a [u64; BANDS],
    idx: usize,
}

impl<'a, const BANDS: usize> BandMatches<'a, BANDS> {
    /// Wrap two band hash arrays into an iterator of matching band indices.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let a = band_hash(&[1u64]);
    /// let b = band_hash(&[2u64]);
    /// let c = band_hash(&[1u64]); // same as `a`
    /// let left = [a, b];
    /// let right = [c, b];
    /// let mut matches = BandMatches::new(&left, &right);
    /// assert_eq!(matches.next(), Some(0));
    /// assert_eq!(matches.next(), Some(1));
    /// assert_eq!(matches.next(), None);
    /// ```
    #[must_use]
    pub fn new(a: &'a [u64; BANDS], b: &'a [u64; BANDS]) -> Self {
        BandMatches { a, b, idx: 0 }
    }
}

impl<const BANDS: usize> Iterator for BandMatches<'_, BANDS> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
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
        (0, Some(BANDS.saturating_sub(self.idx)))
    }
}
