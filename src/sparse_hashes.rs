//! [`SparseHashes`]: MinHash with a bottom-k sparse hash prefix.
//!
//! Wraps a dense [`MinHash`] and adds a bottom-k prefix that stores hash
//! digests in a sorted list under capacity, promoting to the inner
//! signature on overflow. The promoted signature is bit-identical to what a
//! from-scratch `MinHash` on the same input would have produced, so
//! classical banded LSH keeps working after promotion.
//!
//! # Storage layout
//!
//! Holds a single [`MinHash`] whose `words[0]` is the mode flag:
//! `words[0] == 0` means sparse (`words[1..]` is an ascending sorted list of
//! `saturating_add(1)`-encoded digests, zero-terminated). `words[0] != 0`
//! means dense. The dense stream never writes zero to any slot, so the flag
//! is unambiguous, and the wrapper adds no storage over the inner sketch.

use core::hash::Hash as CoreHash;

use serde::{Deserialize, Serialize};

use crate::hasher::{Hasher, SipHashes13};
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::min_hasher::{MinHasher, Outcome};
use crate::minhash::{check_hash_stream, fold_hash_stream_into, MinHash};
use crate::primitive::{Primitive, SparseFor};

/// MinHash with a bottom-k sparse hash prefix that promotes to a dense
/// signature at capacity.
///
/// # Examples
///
/// ```
/// use minhash_rs::prelude::*;
///
/// let mut sketch = SparseHashes::<u64, 128>::new();
/// sketch.insert(42);
/// assert!(sketch.may_contain(42));
///
/// // Promoting yields a bit-identical MinHash.
/// let dense: MinHash<u64, 128> = sketch.into();
/// assert!(dense.may_contain(42));
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct SparseHashes<
    Word,
    const PERMUTATIONS: usize,
    H: Hasher = SipHashes13,
    Hash: HashType = u64,
> where
    Word: SparseFor<Hash>,
    Hash: Primitive<Word>,
{
    inner: MinHash<Word, PERMUTATIONS, H, Hash>,
}

// ─── Debug / Clone / Copy ───────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> core::fmt::Debug
    for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + core::fmt::Debug,
    Hash: Primitive<Word>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SparseHashes")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Clone
    for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Clone,
    Hash: Primitive<Word>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Copy
    for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Copy,
    Hash: Primitive<Word>,
{
}

// ─── Constructor and mode detection ─────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Copy + Maximal + PartialEq,
    Hash: Primitive<Word>,
{
    /// Compile-time assertion that the sketch has enough registers.
    const ASSERT_PERMUTATIONS: () = assert!(
        PERMUTATIONS >= 2,
        "SparseHashes requires at least 2 permutations"
    );

    /// Create a new empty sketch in sparse mode.
    #[must_use]
    pub fn new() -> Self {
        let () = Self::ASSERT_PERMUTATIONS;
        let zero: Word = Hash::ZERO.convert();
        Self {
            inner: MinHash::from_words([zero; PERMUTATIONS]),
        }
    }

    /// Returns `true` if the sketch is still holding the sparse hash list.
    #[inline]
    #[must_use]
    pub fn is_sparse(&self) -> bool {
        self.inner.as_words()[0] == Hash::ZERO.convert()
    }

    /// Returns `true` if the sketch has been promoted to dense mode.
    #[inline]
    #[must_use]
    pub fn is_dense(&self) -> bool {
        !self.is_sparse()
    }

    /// Number of distinct digests currently held in sparse mode. Returns
    /// `0` in dense mode.
    #[must_use]
    pub fn count(&self) -> usize {
        if !self.is_sparse() {
            return 0;
        }
        let zero: Word = Hash::ZERO.convert();
        let words = self.inner.as_words();
        let mut i = PERMUTATIONS - 1;
        while i > 0 {
            if words[i] != zero {
                return i;
            }
            i -= 1;
        }
        0
    }

    /// Number of distinct digests currently held.
    ///
    /// Returns `Some(count)` while the sketch is still in sparse mode, and
    /// `None` once it has densified. Densification discards the retained
    /// digest set in favour of the per-permutation minimum registers, so
    /// the exact count of distinct inputs is unrecoverable from a dense
    /// signature.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut sketch = SparseHashes::<u64, 128>::new();
    /// sketch.insert(42u64);
    /// sketch.insert(7u64);
    /// sketch.insert(42u64); // duplicate
    /// assert_eq!(sketch.distinct_hashes(), Some(2));
    ///
    /// sketch.densify();
    /// assert_eq!(sketch.distinct_hashes(), None);
    /// ```
    #[must_use]
    pub fn distinct_hashes(&self) -> Option<usize> {
        if self.is_sparse() {
            Some(self.count())
        } else {
            None
        }
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Default
    for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Copy + Maximal + PartialEq,
    Hash: Primitive<Word>,
{
    fn default() -> Self {
        Self::new()
    }
}

// ─── Insert / may_contain / densify / promote ───────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
    /// Insert a value into the sketch. See [`Outcome`] for the return
    /// contract.
    pub fn insert<V: CoreHash>(&mut self, value: V) -> Outcome {
        let digest = MinHash::<Word, PERMUTATIONS, H, Hash>::hash_value(value);
        if self.is_sparse() {
            self.sparse_insert_digest(digest)
        } else {
            fold_hash_stream_into(self.inner.as_words_mut(), digest);
            Outcome::Inserted
        }
    }

    /// Returns whether the sketch may contain the provided value.
    #[must_use]
    pub fn may_contain<V: CoreHash>(&self, value: V) -> bool {
        let digest = MinHash::<Word, PERMUTATIONS, H, Hash>::hash_value(value);
        if self.is_sparse() {
            self.sparse_contains_digest(digest)
        } else {
            check_hash_stream(self.inner.as_words(), digest)
        }
    }

    /// Force the sketch into dense mode in place. No-op on already-dense
    /// sketches.
    pub fn densify(&mut self) {
        if !self.is_sparse() {
            return;
        }
        let count = self.count();
        let mut target: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];
        {
            let words = self.inner.as_words();
            for idx in (1..=count).rev() {
                let encoded: Word = words[idx];
                let digest: Hash =
                    <Word as Primitive<Hash>>::convert(encoded).wrapping_sub(Hash::ONE);
                fold_hash_stream_into(&mut target, digest);
            }
        }
        *self.inner.as_words_mut() = target;
    }

    /// Iterate the distinct digests currently held, in ascending order.
    ///
    /// Returns `Some(iterator)` while the sketch is still in sparse mode,
    /// and `None` once it has densified. The iterator yields decoded
    /// [`HashType`] values, undoing the saturating encoding used for the
    /// zero-terminated storage layout.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut sketch = SparseHashes::<u64, 128>::new();
    /// sketch.insert(1u64);
    /// sketch.insert(2u64);
    /// sketch.insert(3u64);
    /// let digests: Vec<u64> =
    ///     sketch.hashes().expect("still sparse").collect();
    /// assert_eq!(digests.len(), 3);
    /// // Digests are stored in ascending order.
    /// assert!(digests.windows(2).all(|w| w[0] < w[1]));
    ///
    /// sketch.densify();
    /// assert!(sketch.hashes().is_none());
    /// ```
    #[must_use]
    pub fn hashes(&self) -> Option<impl Iterator<Item = Hash> + '_> {
        if !self.is_sparse() {
            return None;
        }
        let count = self.count();
        let words = self.inner.as_words();
        Some(
            words[1..=count].iter().map(|&encoded| {
                <Word as Primitive<Hash>>::convert(encoded).wrapping_sub(Hash::ONE)
            }),
        )
    }

    /// Estimate the Jaccard similarity between two `SparseHashes` sketches.
    #[must_use]
    pub fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        if let (true, true) = (self.is_sparse(), other.is_sparse()) {
            self.sparse_jaccard(other)
        } else {
            let mut a = *self;
            let mut b = *other;
            a.densify();
            b.densify();
            a.inner.estimate_jaccard_index(&b.inner)
        }
    }

    /// Consume the sketch and return the equivalent dense [`MinHash`].
    #[must_use]
    pub fn into_minhash(mut self) -> MinHash<Word, PERMUTATIONS, H, Hash> {
        self.densify();
        self.inner
    }

    // ── Private sparse helpers ─────────────────────────────────────────────

    fn sparse_insert_digest(&mut self, digest: Hash) -> Outcome {
        let zero: Word = Hash::ZERO.convert();
        let encoded: Word = digest.saturating_add(Hash::ONE).convert();
        let count = self.count();

        let Err(pos) = self.inner.as_words()[1..=count].binary_search(&encoded) else {
            return Outcome::Duplicate;
        };

        if count >= PERMUTATIONS.saturating_sub(1) {
            self.densify();
            fold_hash_stream_into(self.inner.as_words_mut(), digest);
            return Outcome::Promoted;
        }

        let words = self.inner.as_words_mut();
        words.copy_within((pos + 1)..=count, pos + 2);
        words[pos + 1] = encoded;
        let sentinel = pos + count + 2;
        if sentinel < PERMUTATIONS {
            words[sentinel] = zero;
        }
        Outcome::Inserted
    }

    fn sparse_contains_digest(&self, digest: Hash) -> bool {
        let encoded: Word = digest.saturating_add(Hash::ONE).convert();
        let count = self.count();
        self.inner.as_words()[1..=count]
            .binary_search(&encoded)
            .is_ok()
    }

    fn sparse_jaccard(&self, other: &Self) -> f64 {
        let a_count = self.count();
        let b_count = other.count();

        let a = &self.inner.as_words()[1..=a_count];
        let b = &other.inner.as_words()[1..=b_count];

        let mut ai = 0usize;
        let mut bi = 0usize;
        let mut intersection = 0usize;
        let mut union_count = 0usize;

        while ai < a_count && bi < b_count {
            match a[ai].cmp(&b[bi]) {
                core::cmp::Ordering::Equal => {
                    intersection += 1;
                    union_count += 1;
                    ai += 1;
                    bi += 1;
                }
                core::cmp::Ordering::Less => {
                    union_count += 1;
                    ai += 1;
                }
                core::cmp::Ordering::Greater => {
                    union_count += 1;
                    bi += 1;
                }
            }
        }
        union_count += (a_count - ai) + (b_count - bi);

        if union_count == 0 {
            return 1.0;
        }
        intersection as f64 / union_count as f64
    }
}

// ─── From<SparseHashes> for MinHash ─────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    From<SparseHashes<Word, PERMUTATIONS, H, Hash>> for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
    fn from(sparse: SparseHashes<Word, PERMUTATIONS, H, Hash>) -> Self {
        sparse.into_minhash()
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> crate::min_hasher::sealed::Sealed
    for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash>,
    Hash: Primitive<Word>,
{
}

// ─── MinHasher trait impl ───────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType, V: CoreHash>
    MinHasher<PERMUTATIONS, V> for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
    type Word = Word;
    type Hash = Hash;
    type Hasher = H;

    fn insert(&mut self, value: V) -> Outcome {
        SparseHashes::insert(self, value)
    }

    fn may_contain(&self, value: V) -> bool {
        SparseHashes::may_contain(self, value)
    }

    fn densify(&mut self) {
        SparseHashes::densify(self);
    }

    fn to_dense(&self) -> MinHash<Self::Word, PERMUTATIONS, Self::Hasher, Self::Hash> {
        let mut cloned = *self;
        cloned.densify();
        cloned.inner
    }

    fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        SparseHashes::estimate_jaccard_index(self, other)
    }
}

// ─── FromIterator ───────────────────────────────────────────────────────────

/// Build a `SparseHashes` from any iterator of hashable values, exactly
/// mirroring the `FromIterator` impl on [`MinHash`]:
///
/// ```
/// use minhash_rs::prelude::*;
///
/// let sketch: SparseHashes<u64, 128> = (0u64..30).collect();
/// assert!(sketch.is_sparse());
/// assert_eq!(sketch.count(), 30);
/// assert!(sketch.may_contain(0u64));
/// assert!(!sketch.may_contain(1_000u64));
/// ```
impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType, V: CoreHash>
    core::iter::FromIterator<V> for SparseHashes<Word, PERMUTATIONS, H, Hash>
where
    Word: SparseFor<Hash> + Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        let mut sketch = SparseHashes::<Word, PERMUTATIONS, H, Hash>::new();
        for value in iter {
            sketch.insert(value);
        }
        sketch
    }
}
