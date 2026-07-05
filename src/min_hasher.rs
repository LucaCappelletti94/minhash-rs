//! [`MinHasher`] trait and [`Outcome`] enum.
//!
//! [`MinHasher`] is the sealed trait implemented by every sketch in this crate:
//! [`MinHash`] (dense), [`SparseHashes`](crate::sparse_hashes::SparseHashes)
//! (sparse-hash prefix), and
//! [`SparseValues`](crate::sparse_values::SparseValues) (sparse-value prefix).
//! The seal prevents external types from implementing it, so every conforming
//! implementation is provably one of the three variants ship-tested for
//! state-equivalence and LSH-band consistency in this crate.
//!
//! `P` is the permutation count. `Value` is the input type each impl accepts
//! (defaults to [`u64`]). Two sketches share a [`MinHasher`] impl at matching
//! `(P, Value, Word, Hash, Hasher)` and are Jaccard-comparable through the
//! trait; cross-variant comparisons (for example [`SparseHashes`] against
//! [`SparseValues`]) are done explicitly by densifying both operands through
//! [`MinHasher::to_dense`].

use core::hash::Hash as CoreHash;

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

/// Result of a single [`MinHasher::insert`] call.
///
/// Dense sketches only ever return [`Outcome::Inserted`]. Sparse wrappers
/// return [`Outcome::Duplicate`] on repeated inputs and [`Outcome::Promoted`]
/// on the specific insert that pays the `O(P^2)` sparse-to-dense promotion
/// tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The value was inserted.
    Inserted,
    /// The value was already present. No state changed.
    Duplicate,
    /// The insert triggered a sparse-to-dense promotion. Every subsequent
    /// insert on this sketch is `O(P)` again, matching a from-scratch dense
    /// sketch.
    Promoted,
}

pub(crate) mod sealed {
    /// Private supertrait sealing [`MinHasher`](super::MinHasher) so external
    /// crates cannot implement it. Only the three sketches shipped by this
    /// crate impl `Sealed`. Everything else fails at compile time.
    pub trait Sealed {}
}

/// Sealed trait implemented by every MinHash-family sketch in this crate.
///
/// `P` is the number of permutations. `Value` is the input type each impl
/// accepts. Cross-variant Jaccard is done explicitly by densifying both
/// operands through [`MinHasher::to_dense`] and calling
/// [`MinHasher::estimate_jaccard_index`] on the resulting dense sketches.
///
/// External types cannot implement this trait, because it is sealed via a
/// private supertrait. The following implementation is rejected at compile
/// time:
///
/// ```compile_fail
/// use minhash_rs::prelude::*;
///
/// struct External;
///
/// impl MinHasher<128, u64> for External {
///     type Word = u64;
///     type Hash = u64;
///     type Hasher = SipHashes13;
///     fn insert(&mut self, _value: u64) -> Outcome { Outcome::Inserted }
///     fn may_contain(&self, _value: u64) -> bool { false }
///     fn densify(&mut self) {}
///     fn to_dense(&self) -> MinHash<u64, 128> { MinHash::new() }
///     fn estimate_jaccard_index(&self, _other: &Self) -> f64 { 0.0 }
/// }
/// ```
pub trait MinHasher<const P: usize, Value: CoreHash = u64>: sealed::Sealed + Sized {
    /// Storage word type for the dense signature.
    type Word: Ord + Copy + Maximal + Primitive<Self::Hash>;
    /// Internal hash-stream width.
    type Hash: HashType + Primitive<Self::Word>;
    /// Phantom hasher marker (e.g. `SipHashes13`, `Fnv`).
    type Hasher: Hasher;

    /// Insert a value into the sketch. See [`Outcome`] for the return
    /// contract.
    fn insert(&mut self, value: Value) -> Outcome;

    /// Return `true` if the sketch may contain `value`. Dense sketches
    /// inherit the classical MinHash false-positive profile. Sparse wrappers
    /// are exact while still sparse.
    fn may_contain(&self, value: Value) -> bool;

    /// Force the sketch into its dense representation in place. No-op on
    /// sketches already dense.
    fn densify(&mut self);

    /// Materialise a dense [`MinHash`] equivalent to the current state.
    fn to_dense(&self) -> MinHash<Self::Word, P, Self::Hasher, Self::Hash>;

    /// Estimate the Jaccard similarity between two sketches of the same
    /// variant.
    ///
    /// Sparse-mode wrappers take the exact bottom-`P` merge when both
    /// operands are still under capacity, and fall through to the classical
    /// dense register-agreement fraction otherwise. Cross-variant
    /// comparisons are done by densifying both operands with
    /// [`MinHasher::to_dense`] first.
    ///
    /// # Examples
    ///
    /// Same-variant sparse-sparse takes the exact fast path:
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let a: SparseHashes<u64, 128> = (0u64..30).collect();
    /// let b: SparseHashes<u64, 128> = (15u64..45).collect();
    /// let j = <SparseHashes<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&a, &b);
    /// assert!((j - 15.0 / 45.0).abs() < 1e-9);
    /// ```
    ///
    /// Cross-variant Jaccard is done by densifying both sides explicitly:
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let sh: SparseHashes<u64, 128> = (0u64..30).collect();
    /// let sv: SparseValues<128> = (0u64..30).collect();
    /// let dense_sh: MinHash<u64, 128> = sh.into();
    /// let dense_sv: MinHash<u64, 128> = sv.into();
    /// let j = dense_sh.estimate_jaccard_index(&dense_sv);
    /// assert!((j - 1.0).abs() < 1e-9);
    /// ```
    fn estimate_jaccard_index(&self, other: &Self) -> f64;

    /// Compute `BANDS` band hashes over the dense representation of this
    /// sketch. Sparse operands materialise their dense representation once
    /// per call before hashing.
    ///
    /// `BANDS` must be at least `1` and must evenly divide `P`, enforced at
    /// compile time by [`MinHash::band_hashes`]. Runtime band counts are
    /// not supported. Sweep across a closed set of `const` values instead.
    ///
    /// State equivalence guarantees the sparse output equals the dense
    /// output on the same input:
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let sparse: SparseHashes<u64, 128> = (0u64..30).collect();
    /// let trait_bands: [u64; 16] =
    ///     <SparseHashes<u64, 128> as MinHasher<128, u64>>::band_hashes::<16>(&sparse);
    /// let dense: MinHash<u64, 128> = sparse.into();
    /// let inherent_bands = dense.band_hashes::<16>();
    /// assert_eq!(trait_bands, inherent_bands);
    /// ```
    #[must_use]
    fn band_hashes<const BANDS: usize>(&self) -> [u64; BANDS]
    where
        Self::Word: CoreHash,
    {
        self.to_dense().band_hashes::<BANDS>()
    }
}
