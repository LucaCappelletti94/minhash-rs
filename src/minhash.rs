//! The [`MinHash`] sketch: a fixed array of per-permutation minimum hashes.
//!
//! `MinHash` is strictly the dense Broder `k`-mins signature. Every slot
//! holds the smallest `Word`-narrowed value of `pi_i(digest)` over every
//! inserted digest, and every operation on it is a dense-mode operation.
//!
//! Sparse prefixes live in dedicated wrapper types:
//! [`SparseHashes`](crate::sparse_hashes::SparseHashes) stores hash digests
//! and [`SparseValues`](crate::sparse_values::SparseValues) stores raw
//! integer values under a `dsi-bitstream` instantaneous code. Both wrappers
//! hold an inner `MinHash` and promote in place at capacity, producing a
//! signature bit-identical to a from-scratch `MinHash` on the same input.

use core::hash::{Hash as CoreHash, Hasher as StdHasher};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::hasher::{Hasher, SipHashes13};
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::min_hasher::{MinHasher, Outcome};
use crate::primitive::Primitive;

/// A MinHash sketch: a fixed array of `PERMUTATIONS` minimum hash values.
///
/// Each register holds the smallest `Word`-narrowed value of `pi_i(digest)`
/// observed for permutation `i`. `H` and `Hash` are phantom type parameters
/// naming the hasher marker and the internal hash-stream width, both
/// zero-sized and defaulting to [`SipHashes13`] and [`u64`]. Sketches with
/// mismatched parameters are different Rust types, so cross-config
/// operations are compile errors.
///
/// The dense hash stream never writes `0` to any slot: the `pi_i` output is
/// bumped to `1` on any permutation that would produce a zero. That
/// maintains `words[0] > 0` in dense mode, which the sparse wrappers
/// ([`SparseHashes`](crate::sparse_hashes::SparseHashes),
/// [`SparseValues`](crate::sparse_values::SparseValues)) use as their
/// sparse-mode flag at no extra storage cost.
///
/// # Examples
///
/// ```
/// use minhash_rs::prelude::*;
///
/// // Default hasher (SipHash-1-3) and default hash width (u64):
/// let mut minhash = MinHash::<u64, 128>::new();
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
///
/// // FNV hasher:
/// let mut minhash = MinHash::<u64, 128, Fnv>::new();
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
///
/// // u32 word backed by u32 hash stream: half the storage per register.
/// let mut minhash = MinHash::<u32, 128, SipHashes13, u32>::new();
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
/// ```
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHash<Word, const PERMUTATIONS: usize, H: Hasher = SipHashes13, Hash: HashType = u64>
where
    Hash: Primitive<Word>,
{
    #[serde(with = "BigArray")]
    words: [Word; PERMUTATIONS],

    #[serde(skip)]
    _hasher: PhantomData<H>,

    #[serde(skip)]
    _hash: PhantomData<Hash>,
}

// ─── Debug / Clone / Copy ───────────────────────────────────────────────────

impl<Word: core::fmt::Debug, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> core::fmt::Debug
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MinHash")
            .field("words", &self.words)
            .finish()
    }
}

impl<Word: Clone, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Clone
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            _hasher: PhantomData,
            _hash: PhantomData,
        }
    }
}

impl<Word: Copy, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Copy
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
}

// ─── PartialEq / Eq / core::hash::Hash ──────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> PartialEq
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: PartialEq,
    Hash: Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        self.words == other.words
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Eq
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Eq,
    Hash: Primitive<Word>,
{
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> CoreHash
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: CoreHash,
    Hash: Primitive<Word>,
{
    fn hash<HS: StdHasher>(&self, state: &mut HS) {
        self.words.hash(state);
    }
}

// ─── Default / new / from_words ─────────────────────────────────────────────

impl<Word: Maximal, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Default
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    /// Create a new empty MinHash. Equivalent to [`MinHash::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<Word: Maximal, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    /// Compile-time assertion that the sketch has at least one register.
    const ASSERT_HAS_REGISTERS: () =
        assert!(PERMUTATIONS >= 1, "MinHash requires at least 1 permutation");

    /// Create a new empty MinHash: every register at the [`Maximal`] sentinel.
    ///
    /// A zero-permutation sketch is rejected at compile time.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    /// assert!(minhash.is_empty());
    /// ```
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let _bad = MinHash::<u64, 0>::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let () = Self::ASSERT_HAS_REGISTERS;
        Self::from_words([Word::maximal(); PERMUTATIONS])
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    /// Construct a `MinHash` directly from an owned slot array. Used by the
    /// sparse wrappers to seed their inner `MinHash` with an all-zero
    /// sparse-mode buffer, and by callers who want to lift a raw dense
    /// signature back into the type system.
    #[must_use]
    pub const fn from_words(words: [Word; PERMUTATIONS]) -> Self {
        Self {
            words,
            _hasher: PhantomData,
            _hash: PhantomData,
        }
    }

    /// Borrow the underlying slot array.
    #[must_use]
    pub const fn as_words(&self) -> &[Word; PERMUTATIONS] {
        &self.words
    }

    /// Mutably borrow the underlying slot array. Used by the sparse wrappers
    /// to manipulate `words[0]` (their mode flag) and `words[1..]` (their
    /// codec-encoded prefix) in place.
    #[must_use]
    pub fn as_words_mut(&mut self) -> &mut [Word; PERMUTATIONS] {
        &mut self.words
    }

    /// Consume the sketch and return the raw slot array.
    #[must_use]
    pub fn into_words(self) -> [Word; PERMUTATIONS] {
        self.words
    }
}

// ─── is_empty / is_full ────────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Copy + PartialEq + Maximal,
    Hash: Primitive<Word>,
{
    /// Returns `true` if the sketch is empty: every register at the
    /// [`Maximal`] sentinel, which is the state right after `new`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == Word::maximal())
    }

    /// Returns `true` if the sketch is fully populated: every register
    /// strictly below the [`Maximal`] sentinel.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.words.iter().all(|w| *w != Word::maximal())
    }
}

// ─── Core operations ────────────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
    /// Hash a value with the phantom hasher and narrow to `Hash`.
    #[inline]
    pub(crate) fn hash_value<V: CoreHash>(value: V) -> Hash {
        let mut hasher = H::build();
        value.hash(&mut hasher);
        Hash::from_u64_digest(hasher.finish())
    }

    /// Insert a value into the MinHash.
    ///
    /// Hashes the value under the phantom hasher into a `Hash`-sized digest,
    /// expands the digest through the per-permutation stream, and folds the
    /// result into the signature.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// assert!(!minhash.may_contain(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    pub fn insert<V: CoreHash>(&mut self, value: V) {
        let digest = Self::hash_value(value);
        fold_hash_stream_into(&mut self.words, digest);
    }

    /// Returns whether the MinHash may contain the provided value.
    ///
    /// The dense hash-stream check returns `false` only when at least one
    /// register would have been strictly smaller had `value` been inserted,
    /// which is a proof of absence. `true` is the standard MinHash "may be
    /// present" answer with the classical false-positive profile.
    #[must_use]
    pub fn may_contain<V: CoreHash>(&self, value: V) -> bool {
        let digest = Self::hash_value(value);
        check_hash_stream(&self.words, digest)
    }

    /// Estimate the Jaccard similarity between two dense MinHash sketches.
    ///
    /// Returns the fraction of matching registers, an unbiased estimator of
    /// the Jaccard index under the classical minwise-independence
    /// assumption. Two empty sketches (every register at [`Maximal`])
    /// compare as `1.0` because they hold the same abstract state.
    #[must_use]
    pub fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        dense_jaccard::<Word, PERMUTATIONS>(&self.words, &other.words)
    }

    /// Apply `self[i] = self[i].min(rhs[i])` element-wise. Kept as a tight
    /// indexed loop so LLVM auto-vectorises.
    #[inline]
    pub(crate) fn min_assign(&mut self, rhs: &Self) {
        for i in 0..PERMUTATIONS {
            self.words[i] = self.words[i].min(rhs.words[i]);
        }
    }
}

// ─── Dense hash-stream helpers (crate-visible for wrappers to reuse) ────────

/// Iterator emitting `count` per-slot Word values from the SplitMix +
/// XorShift stream seeded by `seed`. Shared between the fold and check
/// paths and the atomic [`IterHashes`](crate::atomic::IterHashes) surface.
///
/// Zero is never emitted. XorShift has zero as a fixed point, so a stream
/// that ever hits zero would stay there for the rest of the sketch. The
/// guard on the `Hash` value keeps the stream lively, and a second guard on
/// the truncated `Word` prevents the sparse mode flag (`inner.words[0] == 0`
/// on the sparse wrappers) from being accidentally set by a dense stream
/// whose low bits happen to be zero.
pub(crate) struct HashStream<Word, Hash>
where
    Word: Copy + PartialEq,
    Hash: HashType + Primitive<Word>,
{
    hash: Hash,
    zero_word: Word,
    one_word: Word,
    remaining: usize,
    _word: PhantomData<Word>,
}

impl<Word, Hash> HashStream<Word, Hash>
where
    Word: Copy + PartialEq,
    Hash: HashType + Primitive<Word>,
{
    #[inline]
    pub(crate) fn new(digest: Hash, count: usize) -> Self {
        let mut hash = digest.splitmix().splitmix();
        if hash == Hash::ZERO {
            hash = Hash::ONE;
        }
        Self {
            hash,
            zero_word: Hash::ZERO.convert(),
            one_word: Hash::ONE.convert(),
            remaining: count,
            _word: PhantomData,
        }
    }
}

impl<Word, Hash> Iterator for HashStream<Word, Hash>
where
    Word: Copy + PartialEq,
    Hash: HashType + Primitive<Word>,
{
    type Item = Word;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.hash = self.hash.xorshift();
        if self.hash == Hash::ZERO {
            self.hash = Hash::ONE;
        }
        let mut w: Word = self.hash.convert();
        if w == self.zero_word {
            w = self.one_word;
        }
        Some(w)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

/// Fold the permutation stream seeded by `seed` into the target signature
/// via `min` per slot.
#[inline]
pub(crate) fn fold_hash_stream_into<Word, const P: usize, Hash>(target: &mut [Word; P], seed: Hash)
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
    for (word, w) in target
        .iter_mut()
        .zip(HashStream::<Word, Hash>::new(seed, P))
    {
        if w < *word {
            *word = w;
        }
    }
}

/// Return `true` iff an insert of `seed` would leave every slot unchanged.
#[inline]
pub(crate) fn check_hash_stream<Word, const P: usize, Hash>(words: &[Word; P], seed: Hash) -> bool
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
    for (&word, w) in words.iter().zip(HashStream::<Word, Hash>::new(seed, P)) {
        if word > w {
            return false;
        }
    }
    true
}

// ─── Dense Jaccard helper (free function shared with sparse wrappers) ───────

#[inline]
pub(crate) fn dense_jaccard<Word: PartialEq, const P: usize>(a: &[Word; P], b: &[Word; P]) -> f64 {
    if P == 0 {
        // Two zero-register sketches carry the same (empty) information, so
        // treat them as perfectly similar rather than returning NaN. In
        // practice `new`'s compile-time assertion (`PERMUTATIONS >= 1`)
        // makes this branch unreachable, but the guard keeps this helper
        // self-contained.
        return 1.0;
    }
    a.iter()
        .zip(b.iter())
        .map(|(l, r)| usize::from(l == r))
        .sum::<usize>() as f64
        / P as f64
}

// ─── Iterators and accessors ────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    /// Iterate over the register words.
    pub fn iter(&self) -> impl Iterator<Item = &Word> {
        self.words.iter()
    }

    /// Mutable variant of [`iter`](Self::iter).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Word> {
        self.words.iter_mut()
    }

    /// The number of permutations (compile-time constant).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// assert_eq!(MinHash::<u64, 128>::new().number_of_permutations(), 128);
    /// ```
    #[must_use]
    pub const fn number_of_permutations(&self) -> usize {
        PERMUTATIONS
    }

    /// The storage size of the sketch, in bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// assert_eq!(MinHash::<u64, 128>::new().memory(), 128 * 64);
    /// assert_eq!(MinHash::<u32, 128>::new().memory(), 128 * 32);
    /// ```
    #[must_use]
    pub const fn memory(&self) -> usize {
        PERMUTATIONS * core::mem::size_of::<Word>() * 8
    }
}

// ─── AsRef / AsMut / Index / IndexMut ───────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> AsRef<[Word]>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn as_ref(&self) -> &[Word] {
        &self.words
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> AsMut<[Word]>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn as_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Index<usize>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    type Output = Word;

    fn index(&self, index: usize) -> &Self::Output {
        &self.words[index]
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> IndexMut<usize>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.words[index]
    }
}

// ─── MinHasher trait impl (identity) ────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> crate::min_hasher::sealed::Sealed
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType, V: CoreHash>
    MinHasher<PERMUTATIONS, V> for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: HashType + Primitive<Word>,
{
    type Word = Word;
    type Hash = Hash;
    type Hasher = H;

    fn insert(&mut self, value: V) -> Outcome {
        MinHash::insert(self, value);
        Outcome::Inserted
    }

    fn may_contain(&self, value: V) -> bool {
        MinHash::may_contain(self, value)
    }

    fn densify(&mut self) {
        // A `MinHash` is always dense.
    }

    fn to_dense(&self) -> MinHash<Self::Word, PERMUTATIONS, Self::Hasher, Self::Hash> {
        *self
    }

    fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        MinHash::estimate_jaccard_index(self, other)
    }
}
