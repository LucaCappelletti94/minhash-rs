//! LSH banding on MinHash signatures.

use core::hash::{Hash as CoreHash, Hasher};

use fnv::FnvHasher;

use crate::hasher::Hasher as MinHashHasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

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

/// Generic helper carrying the compile-time assertion for
/// [`MinHash::band_hashes`]. Enforcing the bound with an associated const on
/// a dedicated struct lets it depend on both `PERMUTATIONS` (from the
/// sketch) and `BANDS` (from the method), which a plain method-level assert
/// cannot reach on stable Rust before 1.79's inline `const` blocks.
struct AssertBandsDivide<const PERMUTATIONS: usize, const BANDS: usize>;

impl<const PERMUTATIONS: usize, const BANDS: usize> AssertBandsDivide<PERMUTATIONS, BANDS> {
    const OK: () = assert!(
        BANDS >= 1 && PERMUTATIONS % BANDS == 0,
        "band_hashes: BANDS must be at least 1 and must evenly divide PERMUTATIONS"
    );
}

impl<Word, const PERMUTATIONS: usize, H, Hash> MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: CoreHash + Ord + Copy + Maximal + Primitive<Hash>,
    H: MinHashHasher,
    Hash: HashType + Primitive<Word>,
{
    /// Band hashes of this signature: `BANDS` consecutive bands of
    /// `PERMUTATIONS / BANDS` registers each.
    ///
    /// `BANDS` must be at least `1` and must evenly divide `PERMUTATIONS`,
    /// enforced at compile time by an associated-const assertion. `BANDS`
    /// out of range (zero, larger than `PERMUTATIONS`, or non-divisor)
    /// used to silently produce meaningless output. Misuse is now a hard
    /// `E0080` error at the call site.
    ///
    /// If the sketch is in sparse mode it is densified into a temporary
    /// before hashing, so callers do not have to reason about the mode.
    /// Because densification produces a signature bit-identical to what a
    /// from-scratch dense sketch would have produced for the same input
    /// set, banded LSH analysis applies under the same minwise-independence
    /// assumptions as the dense pipeline, regardless of whether the sketch
    /// entered dense mode by direct insertion or by promotion from sparse.
    /// The exact `Pr[band collision] = J^r` law of Broder / Indyk-Motwani
    /// holds under an idealized random-permutation family; the crate's
    /// SplitMix + XorShift permutation stream is a pseudo-permutation, so
    /// downstream analysis carries the standard "assuming minwise
    /// independence" caveat that classical MinHash LSH already carries.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let sketch: MinHash<u64, 128> = (0..100u64).collect();
    /// let hashes = sketch.band_hashes::<16>();
    /// assert_eq!(hashes.len(), 16);
    /// ```
    ///
    /// A `BANDS` value that does not evenly divide `PERMUTATIONS` is
    /// rejected at compile time:
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let sketch: MinHash<u64, 128> = (0..100u64).collect();
    /// let _ = sketch.band_hashes::<13>();
    /// ```
    ///
    /// A `BANDS` value larger than `PERMUTATIONS` is likewise rejected:
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let sketch: MinHash<u64, 128> = (0..100u64).collect();
    /// let _ = sketch.band_hashes::<200>();
    /// ```
    ///
    /// A zero `BANDS` value is likewise rejected:
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let sketch: MinHash<u64, 128> = (0..100u64).collect();
    /// let _ = sketch.band_hashes::<0>();
    /// ```
    #[must_use]
    pub fn band_hashes<const BANDS: usize>(&self) -> [u64; BANDS] {
        // Force compile-time evaluation of the divisibility assertion.
        // Without a consumer of the associated const the assertion is dead
        // code and never fires, exactly the trap the sparse and new asserts
        // fell into pre-fix.
        let () = AssertBandsDivide::<PERMUTATIONS, BANDS>::OK;

        // With `PERMUTATIONS % BANDS == 0` enforced, `rows` is well-defined
        // and every register participates in exactly one band.
        let rows = PERMUTATIONS / BANDS;
        let registers = self.as_ref();
        core::array::from_fn(|band| band_hash(&registers[band * rows..(band + 1) * rows]))
    }
}
