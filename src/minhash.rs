//! Module providing the MinHash data structure.

use crate::{
    prelude::Primitive, primitive::SparseWord, primitive::ToU64, splitmix::SplitMix,
    xorshift::XorShift,
};
use core::hash::{Hash, Hasher};
use core::ops::Index;
use core::ops::IndexMut;
use fnv::FnvHasher;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use siphasher::sip128::SipHasher13;

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
/// [`new()`]: MinHash::new
/// [`sparse()`]: MinHash::sparse
#[repr(transparent)]
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(bound(serialize = "Word: Serialize", deserialize = "Word: Deserialize<'de>"))]
pub struct MinHash<Word, const PERMUTATIONS: usize> {
    #[serde(with = "BigArray")]
    words: [Word; PERMUTATIONS],
}

// ─── PartialEq / Eq / Hash (manual for cross-mode support) ───────────────────

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize> PartialEq
    for MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        match (self.is_sparse(), other.is_sparse()) {
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

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize> Eq
    for MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
}

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal + Hash, const PERMUTATIONS: usize> Hash
    for MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the densified representation so that sparse and dense sketches
        // that compare equal also hash equal (Hash contract: k1 == k2 =>
        // hash(k1) == hash(k2)).
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

impl<Word: Maximal, const PERMUTATIONS: usize> Default for MinHash<Word, PERMUTATIONS> {
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

impl<Word: Maximal, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
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
        }
    }
}

// ─── Sparse constructor (64-bit word types only) ───────────────────────────

impl<Word, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS>
where
    Word: SparseWord + Maximal,
    u64: Primitive<Word>,
{
    // Sparse mode needs at least 2 permutations: one for the mode flag,
    // one for digests. With fewer, the impl block doesn't even exist.
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
    /// after densification. This is the same contract as dense mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::sparse();
    /// minhash.insert_with_siphashes13(42);
    /// assert!(minhash.may_contain_value_with_siphashes13(42));
    /// ```
    #[must_use]
    pub fn sparse() -> Self {
        let zero: Word = 0u64.convert();
        Self {
            words: [zero; PERMUTATIONS],
        }
    }
}

// ─── Mode detection ─────────────────────────────────────────────────────────

impl<Word: PartialEq, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS>
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

impl<Word: Copy + PartialEq + Maximal, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS>
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
    /// minhash.insert_with_siphashes13(42);
    /// assert!(!minhash.is_empty());
    /// ```
    ///
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
    ///    minhash.insert_with_siphashes13(i);
    /// }
    ///
    /// assert!(minhash.is_full());
    /// ```
    ///
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

impl<Word: Ord + XorShift + Copy + ToU64 + Maximal, const PERMUTATIONS: usize>
    MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
    // ── Sparse constructor (gated on 64-bit word types) ─────────────────────

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

        // Find the last digest index (scan from the end for O(1) on full lists).
        let last_idx = self.words[1..]
            .iter()
            .rposition(|&w| w != zero)
            .map_or(0, |i| i + 1);

        // Process digests in reverse order so each is consumed before its
        // slot in `target` is overwritten.
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

    /// Find the number of stored digests via reverse scan.
    /// O(1) when full (last slot non-zero), O(n) when empty.
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

    /// Insert an encoded digest into the sorted list. Densifies on overflow.
    ///
    /// The digest is stored as `digest.wrapping_add(1)` to reserve `0` as
    /// sentinel.
    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_insert_digest(&mut self, digest: u64) {
        let zero: Word = 0u64.convert();
        let encoded: Word = digest.wrapping_add(1).convert();
        let len = self.sparse_len();

        // Single binary search: Ok means duplicate, Err gives insertion pos.
        let Err(pos) = self.words[1..=len].binary_search(&encoded) else {
            return;
        };

        // Check capacity: PERMUTATIONS-1 digests fit in words[1..].
        if len >= PERMUTATIONS.saturating_sub(1) {
            self.densify();
            Self::insert_hash_stream(&mut self.words, digest);
            return;
        }

        // Shift right to make room at words[pos+1].
        self.words.copy_within((pos + 1)..=len, pos + 2);
        self.words[pos + 1] = encoded;

        // Zero sentinel after the new last digest (if room).
        let sentinel = pos + len + 2;
        if sentinel < PERMUTATIONS {
            self.words[sentinel] = zero;
        }
    }

    /// Check if an encoded digest is present in the sorted list.
    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_contains_digest(&self, digest: u64) -> bool {
        let encoded: Word = digest.wrapping_add(1).convert();
        let len = self.sparse_len();
        self.words[1..=len].binary_search(&encoded).is_ok()
    }

    /// Exact Jaccard between two sparse sketches via sorted-list merge walk.
    ///
    /// Compares encoded values directly (wrapping_add(1) preserves order),
    /// avoiding decode overhead. Uses raw index loops instead of iterators.
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

    /// Merge another sparse sketch into this one.
    ///
    /// Merges both sorted digest lists into a single MaybeUninit buffer
    /// (avoiding zero-init overhead), then copies the result back.
    /// On overflow, densifies both operands and falls back to dense union.
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

        // Single merge buffer without zero-init.
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

    /// Insert a hash stream into the given words array.
    ///
    /// This is the hot path shared by all insert methods in dense mode: derive
    /// a seed from SplitMix64, advance through XorShift permutations, and take
    /// the per-register minimum. Zero is remapped to one to reserve zero as the
    /// sparse mode flag.
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

    /// Check whether all words are less than or equal to their corresponding
    /// hash stream values. Shared by all may_contain_value methods in dense
    /// mode. Zero is remapped to one to match the insert path.
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
    // ── Public insert methods (mode dispatch) ───────────────────────────────

    /// Insert a value into the MinHash using the SipHasher13.
    ///
    /// In sparse mode, only the SipHash digest is stored (O(log n) sorted
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
    /// assert!(!minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(42);
    /// assert!(minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(47);
    /// assert!(minhash.may_contain_value_with_siphashes13(47));
    /// ```
    pub fn insert_with_siphashes13<H: Hash>(&mut self, value: H) {
        if self.is_sparse() {
            let mut hasher = SipHasher13::new();
            value.hash(&mut hasher);
            self.sparse_insert_digest(hasher.finish());
        } else {
            let mut hasher = SipHasher13::new();
            value.hash(&mut hasher);
            Self::insert_hash_stream(&mut self.words, hasher.finish());
        }
    }

    /// Insert a value into the MinHash using the keyed SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key0 = 0x0123456789ABCDEF;
    /// let key1 = 0xFEDCBA9876543210;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(42, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(47, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(47, key0, key1));
    /// ```
    pub fn insert_with_keyed_siphashes13<H: Hash>(&mut self, value: H, key0: u64, key1: u64) {
        if self.is_sparse() {
            let mut hasher = SipHasher13::new_with_keys(key0, key1);
            value.hash(&mut hasher);
            self.sparse_insert_digest(hasher.finish());
        } else {
            let mut hasher = SipHasher13::new_with_keys(key0, key1);
            value.hash(&mut hasher);
            Self::insert_hash_stream(&mut self.words, hasher.finish());
        }
    }

    /// Insert a value into the MinHash using the FNV.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    ///
    /// assert!(!minhash.may_contain_value_with_fnv(42));
    /// minhash.insert_with_fnv(42);
    /// assert!(minhash.may_contain_value_with_fnv(42));
    /// minhash.insert_with_fnv(47);
    /// assert!(minhash.may_contain_value_with_fnv(47));
    /// ```
    pub fn insert_with_fnv<H: Hash>(&mut self, value: H) {
        if self.is_sparse() {
            let mut hasher = FnvHasher::default();
            value.hash(&mut hasher);
            self.sparse_insert_digest(hasher.finish());
        } else {
            let mut hasher = FnvHasher::default();
            value.hash(&mut hasher);
            Self::insert_hash_stream(&mut self.words, hasher.finish());
        }
    }

    /// Insert a value into the MinHash using the keyed FNV.
    ///
    /// # Arguments
    /// * `value` - The value to insert.
    /// * `key` - The key.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fnv(42, key));
    /// minhash.insert_with_keyed_fnv(42, key);
    /// assert!(minhash.may_contain_value_with_keyed_fnv(42, key));
    /// minhash.insert_with_keyed_fnv(47, key);
    /// assert!(minhash.may_contain_value_with_keyed_fnv(47, key));
    /// ```
    pub fn insert_with_keyed_fnv<H: Hash>(&mut self, value: H, key: u64) {
        if self.is_sparse() {
            let mut hasher = FnvHasher::with_key(key);
            value.hash(&mut hasher);
            self.sparse_insert_digest(hasher.finish());
        } else {
            let mut hasher = FnvHasher::with_key(key);
            value.hash(&mut hasher);
            Self::insert_hash_stream(&mut self.words, hasher.finish());
        }
    }

    // ── Public may_contain methods (mode dispatch) ──────────────────────────

    /// Returns whether the MinHash may contain the provided value, using the
    /// SipHasher13.
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
    /// assert!(!minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(42);
    /// assert!(minhash.may_contain_value_with_siphashes13(42));
    /// minhash.insert_with_siphashes13(47);
    /// assert!(minhash.may_contain_value_with_siphashes13(47));
    /// ```
    ///
    pub fn may_contain_value_with_siphashes13<H: Hash>(&self, value: H) -> bool {
        if self.is_sparse() {
            let mut hasher = SipHasher13::new();
            value.hash(&mut hasher);
            self.sparse_contains_digest(hasher.finish())
        } else {
            let mut hasher = SipHasher13::new();
            value.hash(&mut hasher);
            Self::check_hash_stream(&self.words, hasher.finish())
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the
    /// keyed SipHasher13.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key0 = 0x0123456789ABCDEF;
    /// let key1 = 0xFEDCBA9876543210;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(42, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
    /// minhash.insert_with_keyed_siphashes13(47, key0, key1);
    /// assert!(minhash.may_contain_value_with_keyed_siphashes13(47, key0, key1));
    /// ```
    ///
    pub fn may_contain_value_with_keyed_siphashes13<H: Hash>(
        &self,
        value: H,
        key0: u64,
        key1: u64,
    ) -> bool {
        if self.is_sparse() {
            let mut hasher = SipHasher13::new_with_keys(key0, key1);
            value.hash(&mut hasher);
            self.sparse_contains_digest(hasher.finish())
        } else {
            let mut hasher = SipHasher13::new_with_keys(key0, key1);
            value.hash(&mut hasher);
            Self::check_hash_stream(&self.words, hasher.finish())
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the
    /// FNV.
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
    /// assert!(!minhash.may_contain_value_with_fnv(42));
    /// minhash.insert_with_fnv(42);
    /// assert!(minhash.may_contain_value_with_fnv(42));
    /// minhash.insert_with_fnv(47);
    /// assert!(minhash.may_contain_value_with_fnv(47));
    /// ```
    ///
    pub fn may_contain_value_with_fnv<H: Hash>(&self, value: H) -> bool {
        if self.is_sparse() {
            let mut hasher = FnvHasher::default();
            value.hash(&mut hasher);
            self.sparse_contains_digest(hasher.finish())
        } else {
            let mut hasher = FnvHasher::default();
            value.hash(&mut hasher);
            Self::check_hash_stream(&self.words, hasher.finish())
        }
    }

    /// Returns whether the MinHash may contain the provided value, using the
    /// keyed FNV.
    ///
    /// # Arguments
    /// * `value` - The value to check.
    /// * `key` - The key.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fnv(42, key));
    /// minhash.insert_with_keyed_fnv(42, key);
    /// assert!(minhash.may_contain_value_with_keyed_fnv(42, key));
    /// minhash.insert_with_keyed_fnv(47, key);
    /// assert!(minhash.may_contain_value_with_keyed_fnv(47, key));
    /// ```
    pub fn may_contain_value_with_keyed_fnv<H: Hash>(&self, value: H, key: u64) -> bool {
        if self.is_sparse() {
            let mut hasher = FnvHasher::with_key(key);
            value.hash(&mut hasher);
            self.sparse_contains_digest(hasher.finish())
        } else {
            let mut hasher = FnvHasher::with_key(key);
            value.hash(&mut hasher);
            Self::check_hash_stream(&self.words, hasher.finish())
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
    /// let first_set: HashSet<u64> = [1_u64, 2_u64, 3_u64, 4_u64, 5_u64, 6_u64, 7_u64, 8_u64].iter().copied().collect();
    /// let second_set: HashSet<u64> = [5_u64, 6_u64, 7_u64, 8_u64, 9_u64, 10_u64, 11_u64, 12_u64].iter().copied().collect();
    ///
    /// let mut first_minhash: MinHash<u64, 128> = first_set.iter().collect();
    /// let mut second_minhash: MinHash<u64, 128> = second_set.iter().collect();
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
        match (self.is_sparse(), other.is_sparse()) {
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
// (min_assign is in the main impl block above; min_assign_dense is here for
// use by union.rs which only needs Ord + Copy.)

impl<Word: Ord + Copy, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Apply element-wise min assuming both operands are dense.
    #[inline]
    pub(crate) fn min_assign_dense(&mut self, rhs: &Self) {
        for i in 0..PERMUTATIONS {
            self.words[i] = self.words[i].min(rhs.words[i]);
        }
    }
}

// ─── Iterators and accessors ────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize> MinHash<Word, PERMUTATIONS> {
    /// Iterate over the words.
    ///
    /// **Sparse mode:** for sparse sketches, this iterates over the raw
    /// internal representation (mode flag at index 0, encoded digests at
    /// indices 1 and beyond). Use [`estimate_jaccard_index`], [`may_contain_value_with_siphashes13`],
    /// or the union operators instead, which handle mode dispatch correctly.
    ///
    /// [`estimate_jaccard_index`]: MinHash::estimate_jaccard_index
    /// [`may_contain_value_with_siphashes13`]: MinHash::may_contain_value_with_siphashes13
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
//
// **Sparse mode note:** these expose the raw internal array. For sparse
// sketches, index 0 is the mode flag (zero) and indices 1+ hold encoded
// digests (wrapping_add(1)). Use the public API methods instead, which
// handle mode dispatch correctly.

impl<Word, const PERMUTATIONS: usize> AsRef<[Word]> for MinHash<Word, PERMUTATIONS> {
    fn as_ref(&self) -> &[Word] {
        &self.words
    }
}

impl<Word, const PERMUTATIONS: usize> AsMut<[Word]> for MinHash<Word, PERMUTATIONS> {
    fn as_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }
}

impl<W, const PERMUTATIONS: usize> Index<usize> for MinHash<W, PERMUTATIONS> {
    type Output = W;

    fn index(&self, index: usize) -> &Self::Output {
        &self.words[index]
    }
}

impl<W, const PERMUTATIONS: usize> IndexMut<usize> for MinHash<W, PERMUTATIONS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.words[index]
    }
}
