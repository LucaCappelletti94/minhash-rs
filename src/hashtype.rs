//! Trait that bundles the operations required of the internal hash stream.
//!
//! MinHash generates its per-permutation hash stream by seeding a small hash
//! type ([`u64`] by default, [`u32`] for half-memory sparse mode) and iterating
//! [`SplitMix`](HashType::splitmix) and [`XorShift`](crate::xorshift::XorShift)
//! on it. Every operation required of that hash type sits on this trait so that
//! the [`MinHash`](crate::minhash::MinHash) machinery is generic in the hash
//! width.

use crate::xorshift::XorShift;

/// Arithmetic operations the sparse mode encoder relies on.
///
/// Sparse mode stores each digest as `digest.saturating_add(ONE)`, using zero
/// as the mode flag and list terminator. Encoding with `saturating_add`
/// prevents `digest == Hash::MAX` from wrapping to zero and colliding with
/// the sentinel; the price is that `digest == MAX` and `digest == MAX - 1`
/// both encode to `MAX`, a benign 1-in-2^N false positive. Decoding uses
/// `wrapping_sub(ONE)`, which recovers every non-sentinel encoded value
/// exactly.
///
/// The wrapping variants are also exposed because the decoder needs them and
/// because a caller writing an alternative sparse-mode routine may want the
/// full pair.
pub trait SparseArithmetic: Sized {
    /// Wrapping addition, matching [`u64::wrapping_add`] semantics.
    #[must_use]
    fn wrapping_add(self, rhs: Self) -> Self;

    /// Wrapping subtraction, matching [`u64::wrapping_sub`] semantics.
    #[must_use]
    fn wrapping_sub(self, rhs: Self) -> Self;

    /// Saturating addition, matching [`u64::saturating_add`] semantics.
    #[must_use]
    fn saturating_add(self, rhs: Self) -> Self;

    /// Saturating subtraction, matching [`u64::saturating_sub`] semantics.
    #[must_use]
    fn saturating_sub(self, rhs: Self) -> Self;
}

impl SparseArithmetic for u64 {
    #[inline]
    fn wrapping_add(self, rhs: Self) -> Self {
        u64::wrapping_add(self, rhs)
    }

    #[inline]
    fn wrapping_sub(self, rhs: Self) -> Self {
        u64::wrapping_sub(self, rhs)
    }

    #[inline]
    fn saturating_add(self, rhs: Self) -> Self {
        u64::saturating_add(self, rhs)
    }

    #[inline]
    fn saturating_sub(self, rhs: Self) -> Self {
        u64::saturating_sub(self, rhs)
    }
}

impl SparseArithmetic for u32 {
    #[inline]
    fn wrapping_add(self, rhs: Self) -> Self {
        u32::wrapping_add(self, rhs)
    }

    #[inline]
    fn wrapping_sub(self, rhs: Self) -> Self {
        u32::wrapping_sub(self, rhs)
    }

    #[inline]
    fn saturating_add(self, rhs: Self) -> Self {
        u32::saturating_add(self, rhs)
    }

    #[inline]
    fn saturating_sub(self, rhs: Self) -> Self {
        u32::saturating_sub(self, rhs)
    }
}

/// A type usable as the internal hash stream width for MinHash.
///
/// The hasher (implementations of [`Hasher`](crate::hasher::Hasher)) always
/// produces a raw [`u64`] digest; the digest is then converted into `Self` via
/// [`from_u64_digest`](Self::from_u64_digest), mixed once through
/// [`splitmix`](Self::splitmix), and iterated with
/// [`XorShift`](crate::xorshift::XorShift) to yield one hash per permutation.
///
/// Two implementations are provided:
/// - [`u64`]: the default, full 64-bit stream.
/// - [`u32`]: 32-bit stream, enabling `u32`-word sparse mode with half the
///   digest storage.
pub trait HashType: XorShift + SparseArithmetic + Copy + Eq {
    /// The additive identity, also the sparse mode flag when converted to a
    /// [`Word`](crate::minhash::MinHash).
    const ZERO: Self;

    /// The multiplicative identity, used to replace the degenerate zero state
    /// of the hash stream.
    const ONE: Self;

    /// The SplitMix finalizer for this hash width.
    ///
    /// The [`u64`] impl is the standard SplitMix64. The [`u32`] impl is a
    /// SplitMix32 variant with constants scaled to 32 bits.
    #[must_use]
    fn splitmix(self) -> Self;

    /// Convert a raw [`u64`] digest emitted by
    /// [`Hasher::finish`](core::hash::Hasher::finish) into this hash width.
    ///
    /// For [`u64`] this is the identity. For [`u32`] this narrows via `as u32`,
    /// which is intentional: the caller has chosen a narrower hash stream.
    #[must_use]
    fn from_u64_digest(digest: u64) -> Self;
}

impl HashType for u64 {
    const ZERO: Self = 0;
    const ONE: Self = 1;

    #[inline]
    fn splitmix(self) -> Self {
        let mut z = self;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    #[inline]
    fn from_u64_digest(digest: u64) -> Self {
        digest
    }
}

impl HashType for u32 {
    const ZERO: Self = 0;
    const ONE: Self = 1;

    #[inline]
    fn splitmix(self) -> Self {
        // Chris Wellons's "lowbias32" mixer, discovered by his prospector
        // search (https://nullprogram.com/blog/2018/07/31/) and released to
        // the public domain. It is a two-round `x ^= x >> s; x *= C;`
        // avalanche mixer with near-zero bias and pairwise-independent
        // avalanche across all 32 bits, which is what MinHash needs from a
        // SplitMix-style finalizer. It is not literally SplitMix32
        // (SplitMix32 has no single canonical form; every 32-bit variant in
        // the wild uses different constants), so this is documented as
        // "lowbias32" rather than as any specific SplitMix descendant.
        let mut z = self;
        z = (z ^ (z >> 16)).wrapping_mul(0x7feb_352d);
        z = (z ^ (z >> 15)).wrapping_mul(0x846c_a68b);
        z ^ (z >> 16)
    }

    #[inline]
    fn from_u64_digest(digest: u64) -> Self {
        digest as Self
    }
}
