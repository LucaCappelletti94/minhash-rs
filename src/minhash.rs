//! Module providing the MinHash data structure.

use crate::{
    hasher::Hasher, prelude::Primitive, primitive::SparseWord, primitive::ToU64,
    splitmix::SplitMix, xorshift::XorShift,
};
use core::hash::{Hash, Hasher as StdHasher};
use core::marker::PhantomData;
use core::ops::Index;
use core::ops::IndexMut;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::hasher::SipHashes13;
use crate::prelude::Maximal;

/// A MinHash sketch: a fixed array of `PERMUTATIONS` minimum hash values.
///
/// The sketch operates in two modes:
///
/// - **Dense mode** (`words[0] != 0`): standard MinHash signature where each
///   register holds the per-permutation minimum hash. Created via [`new()`] or
///   [`Default`].
/// - **Sparse mode** (`words[0] == 0`): stores SipHash/FNV digests in a
///   zero-terminated sorted list at `words[1..]`. Permutation expansion is
///   deferred until densification. Created via [`sparse()`].
///
/// Dense mode works for all supported word types. Sparse mode is only
/// meaningful for `Word = u64` or `usize` on 64-bit platforms (narrow words
/// cannot store full `u64` digests without truncation).
///
/// The third type parameter `H` is a phantom [`Hasher`](crate::hasher::Hasher)
/// marker. Sketches built with different hashers are different Rust types,
/// preventing silent correctness bugs from comparing Jaccard estimates or
/// equality across incompatible hash streams.
///
/// # Examples
///
/// ```
/// use minhash_rs::prelude::*;
///
/// // Default hasher (SipHash-1-3):
/// let mut minhash = MinHash::<u64, 128>::new();
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
///
/// // FNV hasher:
/// let mut minhash = MinHash::<u64, 128, Fnv>::new();
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
///
/// // Keyed SipHash:
/// let mut minhash = MinHash::<u64, 128, SipHashes13Keyed>::new_with_keys(
///     0x0123_4567_89AB_CDEF,
///     0xFEDC_BA98_7654_3210,
/// );
/// minhash.insert(42);
/// assert!(minhash.may_contain(42));
/// ```
///
/// [`new()`]: MinHash::new
/// [`sparse()`]: MinHash::sparse
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHash<Word, const PERMUTATIONS: usize, H: Hasher = SipHashes13> {
    #[serde(with = "BigArray")]
    words: [Word; PERMUTATIONS],

    /// Keys for keyed hashers. `None` for unkeyed hashers or after
    /// deserialization (use `with_keys()` to restore).
    #[serde(skip)]
    keys: Option<[u64; 2]>,

    /// Phantom type parameter for the hasher strategy.
    #[serde(skip)]
    _hasher: PhantomData<H>,
}

impl<Word: Clone, const PERMUTATIONS: usize, H: Hasher> Clone for MinHash<Word, PERMUTATIONS, H> {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            keys: self.keys,
            _hasher: PhantomData,
        }
    }
}

impl<Word: Copy, const PERMUTATIONS: usize, H: Hasher> Copy for MinHash<Word, PERMUTATIONS, H> {}

// ─── PartialEq / Eq / Hash (manual for cross-mode support) ───────────────────

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize, H: Hasher> PartialEq
    for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        match (&self.is_sparse(), &other.is_sparse()) {
            (true, true) | (false, false) => self.words == other.words,
            (true, false) => {
                let mut dense: [Word; PERMUTATIONS] = core::array::from_fn(|_| Word::maximal());
                self.densify_into(&mut dense);
                dense == other.words
            }
            (false, true) => {
                let mut dense: [Word; PERMUTATIONS] = core::array::from_fn(|_| Word::maximal());
                other.densify_into(&mut dense);
                self.words == dense
            }
        }
    }
}

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize, H: Hasher> Eq
    for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
}

impl<
        Word: Ord + XorShift + Copy + ToU64 + Maximal + Hash,
        const PERMUTATIONS: usize,
        H: Hasher,
    > Hash for MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    fn hash<HS: StdHasher>(&self, state: &mut HS) {
        if self.is_sparse() {
            let mut dense: [Word; PERMUTATIONS] = core::array::from_fn(|_| Word::maximal());
            self.densify_into(&mut dense);
            dense.hash(state);
        } else {
            self.words.hash(state);
        }
    }
}

// ─── Default / new (dense) ──────────────────────────────────────────────────

impl<Word: Maximal, const PERMUTATIONS: usize, H: Hasher> Default
    for MinHash<Word, PERMUTATIONS, H>
{
    /// Create a new MinHash with the maximal value.
    ///
    /// The sketch is in dense mode (all words set to the maximal sentinel).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::default();
    ///
    /// assert_eq!(minhash, MinHash::<u64, 128>::new());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl<Word: Maximal, const PERMUTATIONS: usize, H: Hasher> MinHash<Word, PERMUTATIONS, H> {
    /// Create a new MinHash in dense mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            words: [Word::maximal(); PERMUTATIONS],
            keys: None,
            _hasher: PhantomData,
        }
    }

    /// Create a new MinHash in dense mode with explicit keys.
    ///
    /// Only available for keyed hasher types
    /// (`SipHashes13Keyed`, `FnvKeyed`). The keys are stored in the sketch
    /// and used by [`insert()`] and [`may_contain()`] on subsequent calls.
    ///
    /// [`insert()`]: MinHash::insert
    /// [`may_contain()`]: MinHash::may_contain
    ///
    /// # Panics
    ///
    /// Panics if the hasher type does not support keyed construction
    /// (`build_with_keys` returns `None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128, SipHashes13Keyed>::new_with_keys(
    ///     0x0123_4567_89AB_CDEF,
    ///     0xFEDC_BA98_7654_3210,
    /// );
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    #[must_use]
    #[allow(clippy::similar_names)]
    pub fn new_with_keys(key0: u64, key1: u64) -> Self {
        let keys = [key0, key1];
        assert!(
            H::build_with_keys(&keys).is_some(),
            "hasher type does not support keyed construction"
        );
        Self {
            words: [Word::maximal(); PERMUTATIONS],
            keys: Some(keys),
            _hasher: PhantomData,
        }
    }

    /// Set the keys on an existing sketch.
    ///
    /// Useful after deserialization of a keyed sketch, since keys are not
    /// serialized (they are runtime configuration, not sketch data).
    ///
    /// # Panics
    ///
    /// Panics if the hasher type does not support keyed construction.
    #[allow(clippy::similar_names)]
    pub fn with_keys(&mut self, key0: u64, key1: u64) {
        let keys = [key0, key1];
        assert!(
            H::build_with_keys(&keys).is_some(),
            "hasher type does not support keyed construction"
        );
        self.keys = Some(keys);
    }
}

// ─── Sparse constructor (64-bit word types only) ───────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher> MinHash<Word, PERMUTATIONS, H>
where
    Word: SparseWord + Maximal,
    u64: Primitive<Word>,
{
    #[allow(dead_code)]
    const ASSERT_PERMUTATIONS: () = assert!(
        PERMUTATIONS >= 2,
        "sparse mode requires at least 2 permutations"
    );
    /// Create a new MinHash in sparse mode.
    ///
    /// Sparse mode stores SipHash/FNV digests in a sorted list instead of
    /// expanding them through SplitMix + XorShift. Permutation expansion is
    /// deferred until the sketch is densified (automatically on overflow, or
    /// explicitly via operations that require dense representation).
    ///
    /// This method is only available for `Word = u64` or `usize` on 64-bit
    /// platforms, since sparse mode stores full `u64` digests. Narrow word
    /// types (`u8`, `u16`, `u32`) would truncate digests and break injectivity.
    ///
    /// All inserts into a sparse sketch must use the same hasher (same
    /// algorithm and keys); mixing hashers produces a meaningless signature
    /// after densification. This is enforced at the type level by the `H`
    /// parameter.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::sparse();
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    #[must_use]
    pub fn sparse() -> Self {
        let zero: Word = 0u64.convert();
        Self {
            words: [zero; PERMUTATIONS],
            keys: None,
            _hasher: PhantomData,
        }
    }

    /// Create a new MinHash in sparse mode with explicit keys.
    ///
    /// # Panics
    ///
    /// Panics if the hasher type does not support keyed construction.
    #[must_use]
    #[allow(clippy::similar_names)]
    pub fn sparse_with_keys(key0: u64, key1: u64) -> Self {
        let keys = [key0, key1];
        assert!(
            H::build_with_keys(&keys).is_some(),
            "hasher type does not support keyed construction"
        );
        let zero: Word = 0u64.convert();
        Self {
            words: [zero; PERMUTATIONS],
            keys: Some(keys),
            _hasher: PhantomData,
        }
    }
}

// ─── Mode detection ─────────────────────────────────────────────────────────

impl<Word: PartialEq, const PERMUTATIONS: usize, H: Hasher> MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    /// Returns `true` if the sketch is in sparse mode.
    #[inline]
    pub(crate) fn is_sparse(&self) -> bool {
        self.words[0] == 0u64.convert()
    }
}

// ─── is_empty / is_full ─────────────────────────────────────────────────────

impl<Word: Copy + PartialEq + Maximal, const PERMUTATIONS: usize, H: Hasher>
    MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    /// Returns whether the MinHash is empty.
    ///
    /// In sparse mode, the sketch is empty when the digest list has no entries
    /// (`words[0] == 0 && words[1] == 0`). In dense mode, all words are the
    /// maximal sentinel.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    ///
    /// assert!(minhash.is_empty());
    /// minhash.insert(42);
    /// assert!(!minhash.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        if self.is_sparse() {
            self.words[1] == 0u64.convert()
        } else {
            self.iter().all(|word| *word == Word::maximal())
        }
    }

    /// Returns whether the MinHash is fully saturated.
    ///
    /// In sparse mode, the sketch is full when the digest list has reached
    /// capacity (`PERMUTATIONS - 1` digests stored). In dense mode, every word
    /// has reached the smallest hash value the generator can produce (one).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    ///
    /// assert!(!minhash.is_full());
    ///
    /// for i in 0..4096 {
    ///    minhash.insert(i);
    /// }
    ///
    /// assert!(minhash.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        if self.is_sparse() {
            self.words[PERMUTATIONS - 1] != 0u64.convert()
        } else {
            let one: Word = 1u64.convert();
            self.iter().all(|word| *word == one)
        }
    }
}

// ─── Core operations (insert, may_contain, union, Jaccard) ──────────────────

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize, H: Hasher>
    MinHash<Word, PERMUTATIONS, H>
where
    u64: Primitive<Word>,
{
    // ── Densification ───────────────────────────────────────────────────────

    /// Densify the sketch in-place: expand all stored digests through
    /// SplitMix + XorShift and fold into the MinHash signature.
    ///
    /// After densification, the sketch operates in normal dense mode.
    pub(crate) fn densify(&mut self) {
        let mut target: [Word; PERMUTATIONS] = core::array::from_fn(|_| Word::maximal());
        self.densify_into(&mut target);
        self.words = target;
    }

    /// Densify into an existing target array (used by PartialEq and Hash).
    pub(crate) fn densify_into(&self, target: &mut [Word; PERMUTATIONS]) {
        let zero: Word = 0u64.convert();

        let last_idx = self.words[1..]
            .iter()
            .rposition(|&w| w != zero)
            .map_or(0, |i| i + 1);

        for idx in (1..=last_idx).rev() {
            let encoded = self.words[idx];
            let digest: u64 = encoded.to_u64().wrapping_sub(1);

            let zero: Word = 0u64.convert();
            let one: Word = 1u64.convert();
            let mut hash: Word = digest.splitmix().splitmix().convert();
            if hash == zero {
                hash = one;
            }

            let mut update = |t: &mut Word| {
                hash = hash.xorshift();
                if hash == zero {
                    hash = one;
                }
                if hash < *t {
                    *t = hash;
                }
            };
            for t in target.iter_mut() {
                update(t);
            }
        }
    }

    // ── Sparse helpers ──────────────────────────────────────────────────────

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_len(&self) -> usize {
        let zero: Word = 0u64.convert();
        unsafe {
            let ptr = self.words.as_ptr().add(1);
            let mut i = PERMUTATIONS - 1;
            while i > 0 {
                if *ptr.add(i - 1) != zero {
                    return i;
                }
                i -= 1;
            }
        }
        0
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_insert_digest(&mut self, digest: u64) {
        let zero: Word = 0u64.convert();
        let encoded: Word = digest.wrapping_add(1).convert();
        let len = self.sparse_len();

        let Err(pos) = self.words[1..=len].binary_search(&encoded) else {
            return;
        };

        if len >= PERMUTATIONS.saturating_sub(1) {
            self.densify();
            Self::insert_hash_stream(&mut self.words, digest);
            return;
        }

        self.words.copy_within((pos + 1)..=len, pos + 2);
        self.words[pos + 1] = encoded;

        let sentinel = pos + len + 2;
        if sentinel < PERMUTATIONS {
            self.words[sentinel] = zero;
        }
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_contains_digest(&self, digest: u64) -> bool {
        let encoded: Word = digest.wrapping_add(1).convert();
        let len = self.sparse_len();
        self.words[1..=len].binary_search(&encoded).is_ok()
    }

    fn sparse_jaccard(&self, other: &Self) -> f64 {
        let a_len = self.sparse_len();
        let b_len = other.sparse_len();

        let mut ai = 0usize;
        let mut bi = 0usize;
        let mut intersection = 0usize;
        let mut union_count = 0usize;

        let (a, b) = unsafe { (self.words.as_ptr().add(1), other.words.as_ptr().add(1)) };

        while ai < a_len && bi < b_len {
            let aval = unsafe { *a.add(ai) };
            let bval = unsafe { *b.add(bi) };
            match aval.cmp(&bval) {
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
        union_count += (a_len - ai) + (b_len - bi);

        if union_count == 0 {
            return 1.0;
        }
        intersection as f64 / union_count as f64
    }

    fn sparse_union(&mut self, other: &Self) {
        let zero: Word = 0u64.convert();

        let a_len = self.sparse_len();
        let b_len = other.sparse_len();

        let capacity = PERMUTATIONS.saturating_sub(1);

        if a_len + b_len > capacity {
            self.densify();
            let mut other_dense = *other;
            other_dense.densify();
            self.min_assign_dense(&other_dense);
            return;
        }

        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [Word; PERMUTATIONS] =
            unsafe { core::mem::MaybeUninit::<[Word; PERMUTATIONS]>::uninit().assume_init() };

        let (a, b) = unsafe { (self.words.as_ptr().add(1), other.words.as_ptr().add(1)) };

        let (mut ai, mut bi) = (0usize, 0usize);
        let mut wi = 0usize;

        while ai < a_len && bi < b_len {
            let aval = unsafe { *a.add(ai) };
            let bval = unsafe { *b.add(bi) };
            match aval.cmp(&bval) {
                core::cmp::Ordering::Equal => {
                    buf[wi] = aval;
                    ai += 1;
                    bi += 1;
                    wi += 1;
                }
                core::cmp::Ordering::Less => {
                    buf[wi] = aval;
                    ai += 1;
                    wi += 1;
                }
                core::cmp::Ordering::Greater => {
                    buf[wi] = bval;
                    bi += 1;
                    wi += 1;
                }
            }
        }
        while ai < a_len {
            buf[wi] = unsafe { *a.add(ai) };
            ai += 1;
            wi += 1;
        }
        while bi < b_len {
            buf[wi] = unsafe { *b.add(bi) };
            bi += 1;
            wi += 1;
        }

        unsafe {
            let dst = self.words.as_mut_ptr().add(1);
            core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, wi);
        }

        if wi + 1 < PERMUTATIONS {
            self.words[wi + 1] = zero;
        }
    }

    // ── Hash stream (dense mode) ────────────────────────────────────────────

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn insert_hash_stream(words: &mut [Word; PERMUTATIONS], seed: u64) {
        let zero: Word = 0u64.convert();
        let one: Word = 1u64.convert();

        let mut hash: Word = seed.splitmix().splitmix().convert();
        if hash == zero {
            hash = one;
        }

        for word in words {
            hash = hash.xorshift();
            if hash == zero {
                hash = one;
            }
            if hash < *word {
                *word = hash;
            }
        }
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn check_hash_stream(words: &[Word; PERMUTATIONS], seed: u64) -> bool {
        let zero: Word = 0u64.convert();
        let one: Word = 1u64.convert();

        let mut hash: Word = seed.splitmix().splitmix().convert();
        if hash == zero {
            hash = one;
        }

        for &word in words {
            hash = hash.xorshift();
            if hash == zero {
                hash = one;
            }
            if word > hash {
                return false;
            }
        }
        true
    }

    #[inline]
    fn hash_value<V: Hash>(&self, value: V) -> u64 {
        if let Some(keys) = self.keys {
            let mut hasher = H::build_with_keys(&keys).expect(
                "keys are set but hasher does not support \
                 build_with_keys; this is a bug in minhash-rs",
            );
            value.hash(&mut hasher);
            hasher.finish()
        } else {
            let mut hasher = H::build();
            value.hash(&mut hasher);
            hasher.finish()
        }
    }

    // ── Public insert method ────────────────────────────────────────────────

    /// Insert a value into the MinHash.
    ///
    /// The hasher used is determined by the phantom type parameter `H`
    /// (defaulting to [`SipHashes13`]). For keyed hashers, the keys are
    /// stored in the sketch and used automatically.
    ///
    /// In sparse mode, only the hash digest is stored (O(log n) sorted
    /// insert). In dense mode, the full permutation expansion is computed
    /// (O(PERMUTATIONS)).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    ///
    /// assert!(!minhash.may_contain(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// minhash.insert(47);
    /// assert!(minhash.may_contain(47));
    /// ```
    pub fn insert<V: Hash>(&mut self, value: V) {
        let digest = self.hash_value(value);
        if self.is_sparse() {
            self.sparse_insert_digest(digest);
        } else {
            Self::insert_hash_stream(&mut self.words, digest);
        }
    }

    // ── Public may_contain method ───────────────────────────────────────────

    /// Returns whether the MinHash may contain the provided value.
    ///
    /// The hasher used is determined by the phantom type parameter `H`
    /// (defaulting to [`SipHashes13`]).
    ///
    /// In sparse mode, performs an exact binary search on the digest list
    /// (no false positives). In dense mode, checks the MinHash signature
    /// (false positives possible).
    ///
    /// # Arguments
    /// * `value` - The value to check.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    ///
    /// assert!(!minhash.may_contain(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// minhash.insert(47);
    /// assert!(minhash.may_contain(47));
    /// ```
    pub fn may_contain<V: Hash>(&self, value: V) -> bool {
        let digest = self.hash_value(value);
        if self.is_sparse() {
            self.sparse_contains_digest(digest)
        } else {
            Self::check_hash_stream(&self.words, digest)
        }
    }

    // ── Jaccard (mode dispatch) ─────────────────────────────────────────────

    /// Calculate the similarity between two MinHashes.
    ///
    /// When both sketches are in sparse mode, computes the **exact** Jaccard
    /// index on the hash sets (SipHash-13 on `u64` is injective). When both
    /// are dense, uses the standard MinHash approximation. For mixed modes,
    /// the sparse sketch is densified first.
    ///
    /// The two sketches must use the same hasher type (enforced by the type
    /// system via the `H` parameter).
    ///
    /// # Arguments
    /// * `other` - The other MinHash to compare to.
    ///
    /// # Edge cases
    /// Two empty sketches are identical (every word is the maximal sentinel),
    /// so the estimate is `1.0`. The Jaccard index of two empty sets is
    /// undefined (0/0); this method treats identical sketches as perfectly
    /// similar.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let empty = MinHash::<u64, 128>::new();
    /// assert_eq!(empty.estimate_jaccard_index(&empty), 1.0);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashSet;
    /// use minhash_rs::prelude::*;
    ///
    /// let first_set: HashSet<u64> = [1, 2, 3, 4, 5, 6, 7, 8].iter().copied().collect();
    /// let second_set: HashSet<u64> = [5, 6, 7, 8, 9, 10, 11, 12].iter().copied().collect();
    ///
    /// let first_minhash: MinHash<u64, 128> = first_set.iter().collect();
    /// let second_minhash: MinHash<u64, 128> = second_set.iter().collect();
    ///
    /// let approximation = first_minhash.estimate_jaccard_index(&second_minhash);
    /// let ground_truth = first_set.intersection(&second_set).count() as f64 / first_set.union(&second_set).count() as f64;
    ///
    /// assert!((approximation - ground_truth).abs() < 0.01, concat!(
    ///     "We expected the approximation to be close to the ground truth, ",
    ///    "but got an error of {} instead. The ground truth is {} and the approximation is {}."
    ///    ), (approximation - ground_truth).abs(), ground_truth, approximation
    /// );
    /// ```
    pub fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        match (&self.is_sparse(), &other.is_sparse()) {
            (true, true) => self.sparse_jaccard(other),
            (false, false) => dense_jaccard(&self.words, &other.words),
            (true, false) => {
                let mut dense_self = *self;
                dense_self.densify();
                dense_jaccard(&dense_self.words, &other.words)
            }
            (false, true) => {
                let mut dense_other = *other;
                dense_other.densify();
                dense_jaccard(&self.words, &dense_other.words)
            }
        }
    }

    /// Apply `self[i] = self[i].min(rhs[i])` for all permutations.
    /// Uses a tight indexed loop so that LLVM can auto-vectorize.
    ///
    /// If the sketch is sparse, densifies first.
    pub(crate) fn min_assign(&mut self, rhs: &Self) {
        if self.is_sparse() {
            if rhs.is_sparse() {
                self.sparse_union(rhs);
                return;
            }
            self.densify();
        }
        if rhs.is_sparse() {
            let mut rhs_dense = *rhs;
            rhs_dense.densify();
            self.min_assign_dense(&rhs_dense);
        } else {
            self.min_assign_dense(rhs);
        }
    }
}

// ─── Element-wise min (dense) ───────────────────────────────────────────────

impl<Word: Ord + Copy, const PERMUTATIONS: usize, H: Hasher> MinHash<Word, PERMUTATIONS, H> {
    #[inline]
    pub(crate) fn min_assign_dense(&mut self, rhs: &Self) {
        for i in 0..PERMUTATIONS {
            self.words[i] = self.words[i].min(rhs.words[i]);
        }
    }
}

// ─── Iterators and accessors ────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher> MinHash<Word, PERMUTATIONS, H> {
    /// Iterate over the words.
    ///
    /// **Sparse mode:** for sparse sketches, this iterates over the raw
    /// internal representation (mode flag at index 0, encoded digests at
    /// indices 1 and beyond). Use [`estimate_jaccard_index`], [`may_contain`],
    /// or the union operators instead, which handle mode dispatch correctly.
    ///
    /// [`estimate_jaccard_index`]: MinHash::estimate_jaccard_index
    /// [`may_contain`]: MinHash::may_contain
    pub fn iter(&self) -> impl Iterator<Item = &Word> {
        self.words.iter()
    }

    /// Iterate over the words mutably.
    ///
    /// **Sparse mode:** for sparse sketches, this gives mutable access to the
    /// raw internal representation. Writing to these words can corrupt the
    /// sparse digest list. Use the public insert/union methods instead.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Word> {
        self.words.iter_mut()
    }

    /// Returns the number of permutations.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    ///
    /// assert_eq!(minhash.number_of_permutations(), 128);
    /// ```
    pub fn number_of_permutations(&self) -> usize {
        PERMUTATIONS
    }

    /// Returns memory required to store the MinHash in bits.
    ///
    /// # Examples
    ///
    /// For a MinHash with 128 permutations and 64 bit words, the memory required is 128 * 64 * 8.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    ///
    /// assert_eq!(minhash.memory(), 128 * 64);
    /// ```
    ///
    /// For a MinHash with 128 permutations and 32 bit words, the memory required is 128 * 32 * 8.
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u32, 128>::new();
    ///
    /// assert_eq!(minhash.memory(), 128 * 32);
    /// ```
    ///
    pub fn memory(&self) -> usize {
        PERMUTATIONS * core::mem::size_of::<Word>() * 8
    }
}

// ─── Dense Jaccard helper (free function for reuse) ─────────────────────────

#[inline]
fn dense_jaccard<Word: PartialEq, const P: usize>(a: &[Word; P], b: &[Word; P]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(l, r)| usize::from(l == r))
        .sum::<usize>() as f64
        / P as f64
}

// ─── AsRef / AsMut / Index / IndexMut ───────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher> AsRef<[Word]> for MinHash<Word, PERMUTATIONS, H> {
    fn as_ref(&self) -> &[Word] {
        &self.words
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher> AsMut<[Word]> for MinHash<Word, PERMUTATIONS, H> {
    fn as_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher> Index<usize> for MinHash<Word, PERMUTATIONS, H> {
    type Output = Word;

    fn index(&self, index: usize) -> &Self::Output {
        &self.words[index]
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher> IndexMut<usize>
    for MinHash<Word, PERMUTATIONS, H>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.words[index]
    }
}
