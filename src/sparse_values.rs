//! [`SparseValues`]: MinHash with a codec-generic sparse-value prefix.
//!
//! Wraps a [`MinHash<u64, PERMUTATIONS, H, Hash>`] and adds a sparse prefix
//! that stores raw `u64` values under a `dsi-bitstream` instantaneous code
//! (via the `Code` type parameter, defaulting to Elias gamma) instead of
//! pre-hashed digests. On overflow the sketch promotes to a Broder `k`-mins
//! signature bit-identical to what a from-scratch `MinHash` on the same
//! `u64` input would have produced.
//!
//! # Storage layout
//!
//! `words[0] == 0` selects sparse mode. `words[1..]` is reinterpreted as a
//! byte buffer holding a fixed-width count preamble followed by the
//! codec-encoded value stream from [`sketching_core::sparse_value_list`].
//! `words[0] != 0` selects dense mode and every operation delegates to the
//! inner `MinHash`.

use core::marker::PhantomData;

use dsi_bitstream::prelude::{
    BitRead, BitSeek, BitWrite, BufBitReader, BufBitWriter, CodesRead, CodesWrite, MemWordReader,
    MemWordWriterSlice,
};
use sketching_core::sparse_value_list::{
    code_consts, contains_value, insert_value, read_fixed_bits, union_count, write_fixed_bits,
    CodeLen, ConstCode, DynamicCodeRead, DynamicCodeWrite, ValueInsertion, ValueIter, BE,
};

use crate::hasher::{Hasher, SipHashes13};
use crate::hashtype::HashType;
use crate::min_hasher::{MinHasher, Outcome};
use crate::minhash::MinHash;
use crate::primitive::Primitive;

/// Bit budget of the sparse-mode tail (`words[1..]` viewed as bits).
#[inline]
const fn tail_bits(permutations: usize) -> u32 {
    (permutations as u32).saturating_sub(1).saturating_mul(64)
}

/// Width of the sparse-mode count preamble, in bits.
#[inline]
const fn preamble_bits(permutations: usize) -> u32 {
    let n = tail_bits(permutations);
    32u32 - n.leading_zeros()
}

/// MinHash with a codec-generic sparse-value prefix.
///
/// # Examples
///
/// ```
/// use minhash_rs::prelude::*;
///
/// let mut sketch = SparseValues::<128>::new();
/// for value in 0u64..64 {
///     sketch.insert(value);
/// }
/// assert!(sketch.may_contain(0));
/// assert!(sketch.may_contain(63));
/// assert!(!sketch.may_contain(1000));
///
/// let dense: MinHash<u64, 128> = sketch.into();
/// assert!(dense.may_contain(42u64));
/// ```
pub struct SparseValues<
    const PERMUTATIONS: usize,
    H: Hasher = SipHashes13,
    Hash: HashType = u64,
    Code = ConstCode<{ code_consts::GAMMA }>,
> where
    Hash: Primitive<u64>,
{
    inner: MinHash<u64, PERMUTATIONS, H, Hash>,
    _code: PhantomData<Code>,
}

// ─── Debug / Clone / Copy ───────────────────────────────────────────────────

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code> core::fmt::Debug
    for SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SparseValues")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code> Clone
    for SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code> Copy
    for SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
{
}

// ─── Constructor and mode detection ─────────────────────────────────────────

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code>
    SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
{
    const ASSERT_PERMUTATIONS: () = assert!(
        PERMUTATIONS >= 2,
        "SparseValues requires at least 2 permutations"
    );

    /// Bit width of the sparse-mode count preamble.
    pub const PREAMBLE_BITS: u32 = preamble_bits(PERMUTATIONS);

    /// Total bit budget of the sparse-mode tail (preamble + value stream).
    pub const TAIL_BITS: u32 = tail_bits(PERMUTATIONS);

    /// Create a new empty sketch in sparse-value mode.
    #[must_use]
    pub fn new() -> Self {
        let () = Self::ASSERT_PERMUTATIONS;
        Self {
            inner: MinHash::from_words([0u64; PERMUTATIONS]),
            _code: PhantomData,
        }
    }

    /// Returns `true` if the sketch is in sparse-value mode.
    #[inline]
    #[must_use]
    pub fn is_sparse(&self) -> bool {
        self.inner.as_words()[0] == 0
    }

    /// Returns `true` if the sketch has been promoted to dense mode.
    #[inline]
    #[must_use]
    pub fn is_dense(&self) -> bool {
        !self.is_sparse()
    }

    /// The number of permutations (compile-time constant).
    #[inline]
    #[must_use]
    pub const fn number_of_permutations(&self) -> usize {
        PERMUTATIONS
    }

    /// The storage size of the sketch, in bits.
    #[inline]
    #[must_use]
    pub const fn memory(&self) -> usize {
        PERMUTATIONS * 64
    }

    /// Byte view of the sparse-mode tail (`inner.words[1..]`).
    #[inline]
    #[allow(unsafe_code)]
    fn tail_bytes(&self) -> &[u8] {
        // SAFETY: `[u64; PERMUTATIONS]` is `u64`-aligned; starting at
        // `words[1]` gives a properly aligned pointer for byte reads, and
        // the exposed slice covers exactly `8 * (PERMUTATIONS - 1)` bytes.
        // The const assertion enforces `PERMUTATIONS >= 2`.
        unsafe {
            core::slice::from_raw_parts(
                self.inner.as_words().as_ptr().add(1).cast::<u8>(),
                8 * (PERMUTATIONS - 1),
            )
        }
    }

    #[inline]
    #[allow(unsafe_code)]
    fn tail_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: same as `tail_bytes`; exclusive access via `&mut self`.
        unsafe {
            core::slice::from_raw_parts_mut(
                self.inner.as_words_mut().as_mut_ptr().add(1).cast::<u8>(),
                8 * (PERMUTATIONS - 1),
            )
        }
    }
}

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code> Default
    for SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
{
    fn default() -> Self {
        Self::new()
    }
}

// ─── Codec-dependent operations ─────────────────────────────────────────────

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code>
    SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: Primitive<u64>,
    u64: Primitive<Hash>,
    Code: DynamicCodeRead + DynamicCodeWrite + CodeLen + Copy,
    for<'r> BufBitReader<BE, MemWordReader<u64, &'r [u64], true>>:
        CodesRead<BE> + BitSeek + BitRead<BE>,
    for<'w> BufBitWriter<BE, MemWordWriterSlice<u64, &'w mut [u64]>>: CodesWrite<BE> + BitWrite<BE>,
{
    /// Compile-time assertion that `Code` is zero-sized.
    const ASSERT_CODE_IS_ZST: () = assert!(
        core::mem::size_of::<Code>() == 0,
        "SparseValues requires a zero-sized codec marker (e.g. ConstCode<CODE>)"
    );

    /// Construct a fresh instance of the zero-sized codec marker.
    #[inline]
    #[allow(unsafe_code)]
    fn code() -> Code {
        let () = Self::ASSERT_CODE_IS_ZST;
        // SAFETY: `Code` is zero-sized; the all-zero bit pattern is the only
        // valid value and reading it out is sound.
        unsafe { core::mem::MaybeUninit::<Code>::zeroed().assume_init() }
    }

    /// Number of distinct values currently held in sparse mode.
    #[inline]
    #[must_use]
    pub fn count(&self) -> u32 {
        debug_assert!(self.is_sparse(), "count() is only defined in sparse mode");
        #[allow(clippy::cast_possible_truncation)]
        {
            read_fixed_bits(self.tail_bytes(), 0, Self::PREAMBLE_BITS) as u32
        }
    }

    #[inline]
    fn set_count(&mut self, count: u32) {
        write_fixed_bits(
            self.tail_bytes_mut(),
            0,
            Self::PREAMBLE_BITS,
            u64::from(count),
        );
    }

    /// Insert a value into the sketch. See [`Outcome`] for the return
    /// contract.
    pub fn insert(&mut self, value: u64) -> Outcome {
        if self.is_sparse() {
            let count = self.count();
            let code = Self::code();
            let outcome = insert_value::<BE, Code>(
                self.tail_bytes_mut(),
                Self::PREAMBLE_BITS,
                count,
                value,
                code,
            );
            match outcome {
                ValueInsertion::Inserted => {
                    self.set_count(count + 1);
                    Outcome::Inserted
                }
                ValueInsertion::Duplicate => Outcome::Duplicate,
                ValueInsertion::DoesNotFit => {
                    self.densify();
                    MinHash::insert(&mut self.inner, value);
                    Outcome::Promoted
                }
            }
        } else {
            MinHash::insert(&mut self.inner, value);
            Outcome::Inserted
        }
    }

    /// Returns whether the sketch may contain `value`.
    #[must_use]
    pub fn may_contain(&self, value: u64) -> bool {
        if self.is_sparse() {
            let count = self.count();
            contains_value::<BE, Code>(
                self.tail_bytes(),
                Self::PREAMBLE_BITS,
                count,
                value,
                Self::code(),
            )
        } else {
            self.inner.may_contain(value)
        }
    }

    /// Force the sketch into dense mode in place. No-op on already-dense
    /// sketches.
    pub fn densify(&mut self) {
        if !self.is_sparse() {
            return;
        }
        let count = self.count();
        let mut mh = MinHash::<u64, PERMUTATIONS, H, Hash>::new();
        {
            let tail: &[u8] = self.tail_bytes();
            let iter = ValueIter::<BE, Code>::new(tail, Self::PREAMBLE_BITS, count, Self::code());
            for value in iter {
                mh.insert(value);
            }
        }
        self.inner = mh;
    }

    /// Estimate the Jaccard similarity between two `SparseValues` sketches.
    #[must_use]
    pub fn estimate_jaccard_index(&self, other: &Self) -> f64 {
        if let (true, true) = (self.is_sparse(), other.is_sparse()) {
            let a_count = self.count();
            let b_count = other.count();
            let union = union_count::<BE, Code>(
                self.tail_bytes(),
                Self::PREAMBLE_BITS,
                a_count,
                other.tail_bytes(),
                Self::PREAMBLE_BITS,
                b_count,
                Self::code(),
            );
            if union == 0 {
                return 1.0;
            }
            let intersection = u64::from(a_count) + u64::from(b_count) - u64::from(union);
            intersection as f64 / f64::from(union)
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
    pub fn into_minhash(mut self) -> MinHash<u64, PERMUTATIONS, H, Hash> {
        self.densify();
        self.inner
    }
}

// ─── From<SparseValues> for MinHash ─────────────────────────────────────────

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code>
    From<SparseValues<PERMUTATIONS, H, Hash, Code>> for MinHash<u64, PERMUTATIONS, H, Hash>
where
    Hash: Primitive<u64>,
    u64: Primitive<Hash>,
    Code: DynamicCodeRead + DynamicCodeWrite + CodeLen + Copy,
    for<'r> BufBitReader<BE, MemWordReader<u64, &'r [u64], true>>:
        CodesRead<BE> + BitSeek + BitRead<BE>,
    for<'w> BufBitWriter<BE, MemWordWriterSlice<u64, &'w mut [u64]>>: CodesWrite<BE> + BitWrite<BE>,
{
    fn from(sparse: SparseValues<PERMUTATIONS, H, Hash, Code>) -> Self {
        sparse.into_minhash()
    }
}

// ─── MinHasher trait impl ───────────────────────────────────────────────────

impl<const PERMUTATIONS: usize, H: Hasher, Hash: HashType, Code> MinHasher<PERMUTATIONS, u64>
    for SparseValues<PERMUTATIONS, H, Hash, Code>
where
    Hash: HashType + Primitive<u64>,
    u64: Primitive<Hash>,
    Code: DynamicCodeRead + DynamicCodeWrite + CodeLen + Copy,
    for<'r> BufBitReader<BE, MemWordReader<u64, &'r [u64], true>>:
        CodesRead<BE> + BitSeek + BitRead<BE>,
    for<'w> BufBitWriter<BE, MemWordWriterSlice<u64, &'w mut [u64]>>: CodesWrite<BE> + BitWrite<BE>,
{
    type Word = u64;
    type Hash = Hash;
    type Hasher = H;

    fn insert(&mut self, value: u64) -> Outcome {
        SparseValues::insert(self, value)
    }

    fn may_contain(&self, value: u64) -> bool {
        SparseValues::may_contain(self, value)
    }

    fn densify(&mut self) {
        SparseValues::densify(self);
    }

    fn to_dense(&self) -> MinHash<Self::Word, PERMUTATIONS, Self::Hasher, Self::Hash> {
        let mut cloned = *self;
        cloned.densify();
        cloned.inner
    }
}
