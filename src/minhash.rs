//! The [`MinHash`] sketch: a fixed array of per-permutation minimum hashes.

use core::hash::{Hash as CoreHash, Hasher as StdHasher};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::hasher::{Hasher, SipHashes13};
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::primitive::{Primitive, SparseFor};

/// A MinHash sketch: a fixed array of `PERMUTATIONS` minimum hash values.
///
/// The sketch has two modes:
///
/// - **Dense mode** (`words[0] != 0`): the standard MinHash signature, where
///   each register holds the smallest hash observed for its permutation.
///   Created by [`new()`](Self::new) or [`Default`].
/// - **Sparse mode** (`words[0] == 0`): a sorted list of hash digests, stored
///   at `words[1..]` and terminated by zero. Permutation expansion is
///   deferred until the list overflows, at which point the sketch densifies
///   in place. Created by [`sparse()`](Self::sparse); only available when
///   `Word: SparseFor<Hash>`, that is, when the word can hold a full-width
///   digest without loss.
///
/// The third type parameter `H` is a phantom [`Hasher`] marker (defaulting
/// to [`SipHashes13`]); the fourth `Hash` is the internal hash stream width
/// (defaulting to [`u64`]). Both are zero-sized. Sketches built with a
/// different hasher, or a different hash width, are different Rust types, so
/// cross-config equality, Jaccard, and union are compile errors.
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
/// // u32 word backed by u32 hash stream: half the storage per register and
/// // per sparse digest.
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

// ─── Clone / Copy ───────────────────────────────────────────────────────────

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

// ─── PartialEq / Eq / core::hash::Hash (cross-mode aware) ───────────────────

// The equality is *canonical sketch equality after densification*. Two
// sketches compare equal when they hold the same underlying digest set.
// For the sparse-vs-sparse case the sorted encoded lists coincide. For any
// case involving a dense operand the sparse one is densified first and the
// resulting Broder signatures are compared. This is not equality of the
// underlying input sets: two different input sets can still produce equal
// dense signatures by ordinary MinHash collisions, exactly as they can for
// a from-scratch dense sketch. The invariant this impl actually witnesses
// is that a sparse sketch of an input set `S` compares equal to a dense
// sketch built from `S`, because densification produces a bit-identical
// Broder signature.

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> PartialEq
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
    fn eq(&self, other: &Self) -> bool {
        match (self.is_sparse(), other.is_sparse()) {
            (true, true) | (false, false) => self.words == other.words,
            (true, false) => {
                let mut dense: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];
                self.densify_into(&mut dense);
                dense == other.words
            }
            (false, true) => {
                let mut dense: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];
                other.densify_into(&mut dense);
                self.words == dense
            }
        }
    }
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Eq
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
}

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> CoreHash
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash> + CoreHash,
    Hash: Primitive<Word>,
{
    fn hash<HS: StdHasher>(&self, state: &mut HS) {
        if self.is_sparse() {
            let mut dense: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];
            self.densify_into(&mut dense);
            dense.hash(state);
        } else {
            self.words.hash(state);
        }
    }
}

// ─── Default / new ──────────────────────────────────────────────────────────

impl<Word: Maximal, const PERMUTATIONS: usize, H: Hasher, Hash: HashType> Default
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<Word>,
{
    /// Create a new empty (dense) MinHash. Equivalent to [`MinHash::new`].
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
    /// A zero-register MinHash would panic at every `is_sparse`, `insert`,
    /// `may_contain`, `is_empty`, and `is_full` call because those all
    /// touch `words[0]`.
    const ASSERT_HAS_REGISTERS: () =
        assert!(PERMUTATIONS >= 1, "MinHash requires at least 1 permutation");

    /// Create a new empty MinHash in dense mode: every register at the
    /// [`Maximal`] sentinel.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let minhash = MinHash::<u64, 128>::new();
    /// assert!(minhash.is_empty());
    /// ```
    ///
    /// A zero-permutation sketch is rejected at compile time by the
    /// `ASSERT_HAS_REGISTERS` const assertion:
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let _bad = MinHash::<u64, 0>::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        // Force compile-time evaluation of the assertion at every
        // monomorphisation of `new`. Without a use site the const is dead
        // code and never gets to check `PERMUTATIONS >= 1`.
        let () = Self::ASSERT_HAS_REGISTERS;
        Self {
            words: [Word::maximal(); PERMUTATIONS],
            _hasher: PhantomData,
            _hash: PhantomData,
        }
    }
}

// ─── Sparse constructor (round-trip-safe Word/Hash pairs only) ─────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Maximal + Copy + SparseFor<Hash>,
    Hash: Primitive<Word>,
{
    /// Compile-time assertion that the sketch has enough registers for
    /// sparse mode. Sparse mode reserves `words[0]` as the mode flag and
    /// stores digests at `words[1..]`, so `PERMUTATIONS >= 2` gives at
    /// least one storage slot.
    const ASSERT_PERMUTATIONS: () = assert!(
        PERMUTATIONS >= 2,
        "sparse mode requires at least 2 permutations"
    );

    /// Create a new empty MinHash in sparse mode.
    ///
    /// Sparse mode stores hash digests in a sorted list at `words[1..]`
    /// (with `words[0] == 0` acting as the sparse mode flag) instead of
    /// expanding each insert into the full per-permutation minimum
    /// signature up front. Membership and Jaccard on sparse-vs-sparse
    /// pairs are quasi-exact on the underlying digest sets. Once the
    /// digest list fills up the sketch densifies in place and behaves as a
    /// standard MinHash.
    ///
    /// Only available when [`SparseFor<Hash>`] is implemented for `Word`
    /// so that the digest storage round-trips losslessly. The gate
    /// resolves to pairs where `sizeof(Word) >= sizeof(Hash)`.
    ///
    /// # Densification is state-equivalent
    ///
    /// The densified sketch is bit-identical to what a from-scratch dense
    /// sketch built by inserting the same input set from the start would
    /// have produced. This is why classical banded LSH via
    /// [`band_hashes`](Self::band_hashes) works unchanged on any dense (or
    /// densified) sketch: after promotion the sketch is a standard Broder
    /// signature.
    ///
    /// # Caveats
    ///
    /// **Quasi-exact, not exact on original elements.** Sparse-mode Jaccard
    /// is exact on the underlying digest sets. It differs from Jaccard on
    /// the raw input sets by two `O(1 / 2^N)` effects: (a) ordinary
    /// digest-function collisions between distinct input elements, and (b)
    /// the encoding's `saturating_add(1)` step, which collapses `digest
    /// == Hash::MAX` with `digest == Hash::MAX - 1` into the same encoded
    /// slot. Both effects are negligible for `Hash = u64` and small but
    /// worth noting for `Hash = u32`.
    ///
    /// **Densification is a latency spike, not a smooth curve.** The
    /// specific insert that triggers overflow pays an
    /// `O(PERMUTATIONS * PERMUTATIONS)` tax to loop through the sorted
    /// digest list and fold each stored digest through the full permutation
    /// stream. Every subsequent insert is `O(PERMUTATIONS)` again, matching
    /// a from-scratch dense sketch. In a real-time streaming pipeline this
    /// is a deterministic latency spike on one record, so callers that need
    /// smooth tail latency should either pre-densify with
    /// [`new`](Self::new) or budget the spike explicitly.
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
    ///
    /// A sparse sketch with fewer than two permutations is rejected at
    /// compile time by the `ASSERT_PERMUTATIONS` const assertion:
    ///
    /// ```compile_fail
    /// use minhash_rs::prelude::*;
    ///
    /// let _bad = MinHash::<u64, 1>::sparse();
    /// ```
    #[must_use]
    pub fn sparse() -> Self {
        // Force compile-time evaluation of the assertion at every
        // monomorphisation of `sparse`. Without a use site the const is
        // dead code and `MinHash::<_, 1>::sparse()` would silently
        // construct a broken sketch that densifies on the first insert.
        let () = Self::ASSERT_PERMUTATIONS;
        let zero: Word = Hash::ZERO.convert();
        Self {
            words: [zero; PERMUTATIONS],
            _hasher: PhantomData,
            _hash: PhantomData,
        }
    }
}

// ─── Mode detection ────────────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Copy + PartialEq,
    Hash: Primitive<Word>,
{
    /// Returns `true` if the sketch is in sparse mode.
    ///
    /// The dense hash stream guarantees no register is ever set to zero (a
    /// post-conversion guard replaces zero with one), so `words[0] == 0` is a
    /// reliable mode flag: it is the sparse encoding of an empty digest list.
    /// For word/hash pairs where sparse mode is not available (no
    /// [`SparseFor`] impl), the sketch can never enter sparse mode and this
    /// method always returns `false`.
    #[inline]
    #[must_use]
    pub fn is_sparse(&self) -> bool {
        self.words[0] == Hash::ZERO.convert()
    }
}

// ─── is_empty / is_full ────────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Copy + PartialEq + Maximal,
    Hash: Primitive<Word>,
{
    /// Returns whether the sketch has no elements.
    ///
    /// In sparse mode, the sketch is empty when the digest list at
    /// `words[1..]` starts with a zero. In dense mode, when every register is
    /// still the [`Maximal`] sentinel.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    /// assert!(minhash.is_empty());
    /// minhash.insert(42);
    /// assert!(!minhash.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        if self.is_sparse() {
            self.words[1] == Hash::ZERO.convert()
        } else {
            self.words.iter().all(|w| *w == Word::maximal())
        }
    }

    /// Returns whether the sketch is fully saturated.
    ///
    /// In sparse mode, when the digest list has reached capacity
    /// (`PERMUTATIONS - 1` digests, so the final slot is non-zero). In dense
    /// mode, when every register holds the smallest reachable hash value
    /// (one, because the stream's zero-guard replaces zero with one).
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u8, 16>::new();
    /// assert!(!minhash.is_full());
    ///
    /// for i in 0..4096u64 {
    ///     minhash.insert(i);
    /// }
    /// assert!(minhash.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        if self.is_sparse() {
            self.words[PERMUTATIONS - 1] != Hash::ZERO.convert()
        } else {
            let one: Word = Hash::ONE.convert();
            self.words.iter().all(|w| *w == one)
        }
    }
}

// ─── Core operations (insert, may_contain, jaccard, union, sparse/dense
// helpers) ────────────────────────────────────────────────────────────────

impl<Word, const PERMUTATIONS: usize, H: Hasher, Hash: HashType>
    MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Ord + Copy + Maximal + Primitive<Hash>,
    Hash: Primitive<Word>,
{
    // ── Hash generation ─────────────────────────────────────────────────────

    #[inline]
    fn hash_value<V: CoreHash>(value: V) -> Hash {
        let mut hasher = H::build();
        value.hash(&mut hasher);
        Hash::from_u64_digest(hasher.finish())
    }

    // ── Densification ───────────────────────────────────────────────────────

    /// Expand every stored sparse digest into a full permutation stream and
    /// fold it into the dense signature, then replace the sketch's storage
    /// with the resulting dense words.
    ///
    /// A no-op for sketches already in dense mode: the outer callers guard on
    /// [`is_sparse`](Self::is_sparse).
    pub(crate) fn densify(&mut self) {
        let mut target: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];
        self.densify_into(&mut target);
        self.words = target;
    }

    /// Densify into an externally provided target array. Used by
    /// [`PartialEq`], [`CoreHash`], and mixed-mode
    /// [`estimate_jaccard_index`](Self::estimate_jaccard_index) so they can
    /// materialize a dense view without mutating `self`.
    pub(crate) fn densify_into(&self, target: &mut [Word; PERMUTATIONS]) {
        let zero_word: Word = Hash::ZERO.convert();

        let last_idx = self.words[1..]
            .iter()
            .rposition(|w| *w != zero_word)
            .map_or(0, |i| i + 1);

        for idx in (1..=last_idx).rev() {
            let encoded: Word = self.words[idx];
            let digest: Hash = <Word as Primitive<Hash>>::convert(encoded).wrapping_sub(Hash::ONE);
            Self::fold_hash_stream_into(target, digest);
        }
    }

    // ── Sparse helpers ──────────────────────────────────────────────────────

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_len(&self) -> usize {
        let zero_word: Word = Hash::ZERO.convert();
        // The sparse list at words[1..] is populated left-to-right and
        // zero-terminated, so a right-to-left scan for the first non-zero
        // word gives the length in O(len) rather than O(PERMUTATIONS).
        // SAFETY: the pointer walks through the sketch's own storage, never
        // past `words[PERMUTATIONS - 1]`.
        unsafe {
            let ptr = self.words.as_ptr().add(1);
            let mut i = PERMUTATIONS - 1;
            while i > 0 {
                if *ptr.add(i - 1) != zero_word {
                    return i;
                }
                i -= 1;
            }
        }
        0
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_insert_digest(&mut self, digest: Hash) {
        let zero_word: Word = Hash::ZERO.convert();
        let encoded: Word = digest.saturating_add(Hash::ONE).convert();
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
            self.words[sentinel] = zero_word;
        }
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn sparse_contains_digest(&self, digest: Hash) -> bool {
        let encoded: Word = digest.saturating_add(Hash::ONE).convert();
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

        // SAFETY: `add(1)` skips the sparse mode flag; subsequent adds stay
        // inside `words[1..=len]`, and each `len` is bounded by
        // `PERMUTATIONS - 1`.
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
        let zero_word: Word = Hash::ZERO.convert();

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

        // Initialise the merge scratch buffer with `Word::maximal()`. The
        // merge writes the first `wi` slots and `copy_nonoverlapping` reads
        // only that prefix, so the initial values there are inert; the fill
        // exists so this method stays free of `MaybeUninit::assume_init`,
        // which would be UB for any external `Word` impl whose valid bit
        // patterns are restricted.
        let mut buf: [Word; PERMUTATIONS] = [Word::maximal(); PERMUTATIONS];

        // SAFETY: same argument as `sparse_jaccard`; both pointers walk their
        // owning sketch's own words.
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

        // SAFETY: `wi <= a_len + b_len <= capacity < PERMUTATIONS`, so
        // `dst..dst+wi` stays inside the sparse region of the sketch.
        unsafe {
            let dst = self.words.as_mut_ptr().add(1);
            core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, wi);
        }

        if wi + 1 < PERMUTATIONS {
            self.words[wi + 1] = zero_word;
        }
    }

    // ── Dense hash stream (fold a single digest into a signature) ───────────

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn seed_stream(seed: Hash) -> Hash {
        let mut hash = seed.splitmix().splitmix();
        if hash == Hash::ZERO {
            hash = Hash::ONE;
        }
        hash
    }

    /// Fold the permutation stream seeded by `seed` into the target signature.
    ///
    /// The Word-level zero guard is critical: without it, a stream that
    /// narrows to a zero `Word` could park `words[0] == 0` and turn a dense
    /// sketch into a spurious sparse one on the next `is_sparse` check.
    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn fold_hash_stream_into(target: &mut [Word; PERMUTATIONS], seed: Hash) {
        let zero_word: Word = Hash::ZERO.convert();
        let one_word: Word = Hash::ONE.convert();

        let mut hash = Self::seed_stream(seed);

        for word in target.iter_mut() {
            hash = hash.xorshift();
            if hash == Hash::ZERO {
                hash = Hash::ONE;
            }
            let mut w: Word = hash.convert();
            if w == zero_word {
                w = one_word;
            }
            if w < *word {
                *word = w;
            }
        }
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn insert_hash_stream(words: &mut [Word; PERMUTATIONS], seed: Hash) {
        Self::fold_hash_stream_into(words, seed);
    }

    #[inline(always)]
    #[allow(clippy::inline_always)]
    fn check_hash_stream(words: &[Word; PERMUTATIONS], seed: Hash) -> bool {
        let zero_word: Word = Hash::ZERO.convert();
        let one_word: Word = Hash::ONE.convert();

        let mut hash = Self::seed_stream(seed);

        for &word in words {
            hash = hash.xorshift();
            if hash == Hash::ZERO {
                hash = Hash::ONE;
            }
            let mut w: Word = hash.convert();
            if w == zero_word {
                w = one_word;
            }
            if word > w {
                return false;
            }
        }
        true
    }

    // ── Public insert / may_contain ─────────────────────────────────────────

    /// Insert a value into the MinHash.
    ///
    /// Sparse mode records only the hash digest into the sorted list
    /// (O(log n) binary insert). Dense mode expands the digest through the
    /// full permutation stream and folds each element into the signature
    /// (O(PERMUTATIONS)).
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
        if self.is_sparse() {
            self.sparse_insert_digest(digest);
        } else {
            Self::insert_hash_stream(&mut self.words, digest);
        }
    }

    /// Returns whether the MinHash may contain the provided value.
    ///
    /// Sparse mode performs an exact binary search on the digest list, so
    /// membership is exact (no false positives). Dense mode compares the
    /// signature to the value's permutation stream and returns `false` only
    /// when at least one register would have been strictly smaller had the
    /// value been inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    pub fn may_contain<V: CoreHash>(&self, value: V) -> bool {
        let digest = Self::hash_value(value);
        if self.is_sparse() {
            self.sparse_contains_digest(digest)
        } else {
            Self::check_hash_stream(&self.words, digest)
        }
    }

    // ── Jaccard (mode dispatch) ─────────────────────────────────────────────

    /// Estimate the Jaccard similarity between two MinHash sketches.
    ///
    /// When both sketches are sparse, the estimate is the **quasi-exact**
    /// Jaccard index on the underlying digest sets. Two sources of
    /// imprecision compared to the Jaccard of the raw input sets: (a) the
    /// underlying hasher may map two distinct input elements to the same
    /// digest, contributing an ordinary hash-collision false positive, and
    /// (b) the sparse encoding saturates at `Hash::MAX`, collapsing
    /// `digest == MAX` with `digest == MAX - 1` into the same encoded slot.
    /// Both effects are `O(1 / 2^N)` where `N` is the hash width and are
    /// negligible for `Hash = u64`. The second effect is what the CHANGELOG
    /// flags as a benign 1-in-`2^N` false positive.
    ///
    /// When both sketches are dense, the estimate is the fraction of
    /// matching registers. Under an idealized minwise-independent
    /// permutation family, this is an unbiased estimator of the Jaccard
    /// index with variance approximately `1 / PERMUTATIONS`. The crate's
    /// `SplitMix + XorShift` permutation stream is a pseudo-permutation
    /// that approximates such a family under the same standard assumptions
    /// that classical MinHash carries.
    ///
    /// Mixed-mode comparisons densify the sparse operand first. Because
    /// densification is state-equivalent (the densified sketch is
    /// bit-identical to what a from-scratch dense sketch would have
    /// produced for the same underlying set), the mixed-mode result is the
    /// same as the dense-dense result would have been if the sparse operand
    /// had been built dense from the start.
    ///
    /// Two empty sketches compare equal (every register is the [`Maximal`]
    /// sentinel), so this method returns `1.0` for them. The mathematical
    /// Jaccard index of two empty sets is undefined (0/0), and treating the
    /// pair as fully similar makes downstream code simpler.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashSet;
    /// use minhash_rs::prelude::*;
    ///
    /// let left: HashSet<u64> = [1, 2, 3, 4, 5, 6, 7, 8].iter().copied().collect();
    /// let right: HashSet<u64> = [5, 6, 7, 8, 9, 10, 11, 12].iter().copied().collect();
    ///
    /// let a: MinHash<u64, 128> = left.iter().collect();
    /// let b: MinHash<u64, 128> = right.iter().collect();
    ///
    /// let estimate = a.estimate_jaccard_index(&b);
    /// let truth = left.intersection(&right).count() as f64
    ///     / left.union(&right).count() as f64;
    /// assert!((estimate - truth).abs() < 0.05);
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

    // ── Union (min_assign, mode dispatch) ──────────────────────────────────

    /// Apply `self[i] = self[i].min(rhs[i])` element-wise, first densifying
    /// either operand that is still sparse.
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

    /// Dense element-wise min. Kept as a tight indexed loop so LLVM can
    /// auto-vectorize.
    #[inline]
    fn min_assign_dense(&mut self, rhs: &Self) {
        for i in 0..PERMUTATIONS {
            self.words[i] = self.words[i].min(rhs.words[i]);
        }
    }
}

// ─── Dense Jaccard helper (free function so PartialEq and jaccard reuse it) ─

#[inline]
fn dense_jaccard<Word: PartialEq, const P: usize>(a: &[Word; P], b: &[Word; P]) -> f64 {
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
    /// Iterate over the underlying words.
    ///
    /// **Sparse mode:** the iterator walks the raw internal representation
    /// (mode flag at index 0, encoded digests at indices 1..). Callers that
    /// want a dense view should densify first via one of the higher-level
    /// methods ([`estimate_jaccard_index`](Self::estimate_jaccard_index),
    /// [`may_contain`](Self::may_contain), the union operators).
    pub fn iter(&self) -> impl Iterator<Item = &Word> {
        self.words.iter()
    }

    /// Mutable variant of [`iter`](Self::iter).
    ///
    /// **Sparse mode:** writes here can corrupt the sorted digest list; use
    /// the public [`insert`](Self::insert) / union path instead.
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
    pub fn number_of_permutations(&self) -> usize {
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
    pub fn memory(&self) -> usize {
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
