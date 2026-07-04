//! Width conversions between hash-stream and word types.
//!
//! The MinHash pipeline needs to move values across three widths: the raw
//! [`u64`] digest from the hasher, a [`HashType`](crate::hashtype::HashType)
//! value that carries the SplitMix + XorShift stream, and the [`Word`] used
//! for storage. This module bundles the two abstractions that turn those
//! movements into trait bounds.
//!
//! - [`Primitive<T>`] is a plain [`as`]-cast conversion from `Self` to `T`. It
//!   is implemented for every widening or narrowing pair the crate actually
//!   uses. The direction and losslessness are context-dependent: dense mode
//!   uses it to narrow a `HashType` into a smaller `Word`; sparse mode uses
//!   the reverse impls to decode a stored `Word` back into a `HashType`.
//!
//! - [`SparseFor<Hash>`] is a marker for `(Word, Hash)` pairs where the
//!   `Word` is wide enough to hold a full `Hash` digest without loss. Sparse
//!   mode is gated on this trait, and only pairs with
//!   `sizeof(Word) >= sizeof(Hash)` receive an impl.

use crate::hashtype::HashType;

/// A width conversion from `Self` into `T`, implemented as an [`as`]-cast.
///
/// The trait is direction-agnostic: the same trait carries both narrowing
/// (`u64: Primitive<u8>`, used by dense mode when the word is narrower than
/// the hash stream) and widening (`u32: Primitive<u64>`, used to widen a
/// [`u32`] hash into a [`u64`] word when sparse mode round-trips the digest).
///
/// Whether a specific impl is lossy depends on the pair; the caller is
/// responsible for using an impl in a context where the round-trip is
/// meaningful.
pub trait Primitive<T> {
    /// Convert `self` into the target width via an [`as`]-cast.
    fn convert(self) -> T;
}

// ─── Sources: u64 (default HashType) ─────────────────────────────────────

impl Primitive<u8> for u64 {
    #[inline]
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for u64 {
    #[inline]
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for u64 {
    #[inline]
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for u64 {
    #[inline]
    fn convert(self) -> u64 {
        self
    }
}

impl Primitive<usize> for u64 {
    #[inline]
    fn convert(self) -> usize {
        self as usize
    }
}

// ─── Sources: u32 (narrow HashType) ──────────────────────────────────────

impl Primitive<u8> for u32 {
    #[inline]
    fn convert(self) -> u8 {
        self as u8
    }
}

impl Primitive<u16> for u32 {
    #[inline]
    fn convert(self) -> u16 {
        self as u16
    }
}

impl Primitive<u32> for u32 {
    #[inline]
    fn convert(self) -> u32 {
        self
    }
}

impl Primitive<u64> for u32 {
    #[inline]
    fn convert(self) -> u64 {
        u64::from(self)
    }
}

impl Primitive<usize> for u32 {
    #[inline]
    fn convert(self) -> usize {
        self as usize
    }
}

// ─── Sources: narrow Words (needed so sparse machinery type-checks) ─────
//
// These impls make `Word: Primitive<Hash>` universally satisfied, which lets
// the sparse code path compile for narrow-word MinHash. Narrow-word MinHash
// can never actually enter sparse mode (there is no `sparse()` constructor
// for it because `SparseFor` is not implemented for those pairs), so these
// impls are only exercised by dead branches; the sparse-mode invariants
// therefore never depend on them.

impl Primitive<u32> for u8 {
    #[inline]
    fn convert(self) -> u32 {
        u32::from(self)
    }
}

impl Primitive<u64> for u8 {
    #[inline]
    fn convert(self) -> u64 {
        u64::from(self)
    }
}

impl Primitive<u32> for u16 {
    #[inline]
    fn convert(self) -> u32 {
        u32::from(self)
    }
}

impl Primitive<u64> for u16 {
    #[inline]
    fn convert(self) -> u64 {
        u64::from(self)
    }
}

// ─── Sources: usize (sparse decoding on 64-bit; narrow storage on 32-bit) ──

impl Primitive<u32> for usize {
    #[inline]
    fn convert(self) -> u32 {
        self as u32
    }
}

impl Primitive<u64> for usize {
    #[inline]
    fn convert(self) -> u64 {
        self as u64
    }
}

/// Marker for `(Word, Hash)` pairs whose storage is wide enough for sparse
/// mode.
///
/// Sparse mode stores an encoded `Hash` digest as a `Word` and later decodes
/// it. The round-trip is lossless only when `sizeof(Word) >= sizeof(Hash)`,
/// which this trait witnesses. The [`sparse`](crate::minhash::MinHash::sparse)
/// constructor and every operation that depends on decoding stored digests
/// requires `Word: SparseFor<Hash>`.
///
/// Implemented for:
/// - `(u64, u64)`, `(u32, u32)`: identity.
/// - `(u64, u32)`, `(usize, u32)`: widen the [`u32`] digest into the wider
///   word.
/// - `(usize, u64)` on 64-bit targets: identity in practice (`usize == u64`).
pub trait SparseFor<Hash: HashType>: Copy {}

impl SparseFor<u64> for u64 {}
impl SparseFor<u32> for u64 {}

impl SparseFor<u32> for u32 {}

impl SparseFor<u32> for usize {}

#[cfg(target_pointer_width = "64")]
impl SparseFor<u64> for usize {}
