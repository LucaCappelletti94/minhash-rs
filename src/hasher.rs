//! Hasher marker types for compile-time hasher identity on [`MinHash`].
//!
//! The [`Hasher`] trait is implemented by zero-sized marker structs that name
//! a hash strategy. Each marker becomes a phantom type parameter on
//! [`MinHash`](crate::minhash::MinHash), so sketches built with different
//! hashers are different Rust types. That turns cross-hasher comparisons,
//! Jaccard estimates, and unions into compile errors instead of silent
//! correctness bugs.
//!
//! Two markers ship with the crate: [`SipHashes13`] (the default) and
//! [`Fnv`]. Keyed hashing is not exposed; a caller who needs a keyed digest
//! stream can hash a `(key, value)` tuple with either marker instead.

/// A hash strategy that can build a concrete [`core::hash::Hasher`].
///
/// The concrete hasher always emits a raw [`u64`] digest through
/// [`finish`](core::hash::Hasher::finish); the digest is then narrowed into the
/// [`HashType`](crate::hashtype::HashType) chosen for the sketch.
pub trait Hasher {
    /// The concrete [`core::hash::Hasher`] built by [`build`](Self::build).
    type Concrete: core::hash::Hasher;

    /// Construct a fresh hasher instance.
    #[must_use]
    fn build() -> Self::Concrete;
}

/// [SipHash-1-3](https://en.wikipedia.org/wiki/SipHash) with the standard
/// zero-initialised keys. This is the default hasher for
/// [`MinHash`](crate::minhash::MinHash).
#[derive(Debug, Clone, Copy)]
pub struct SipHashes13;

impl Hasher for SipHashes13 {
    type Concrete = siphasher::sip128::SipHasher13;

    #[inline]
    fn build() -> Self::Concrete {
        siphasher::sip128::SipHasher13::new()
    }
}

/// [FNV-1a](https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function)
/// with the default seed. Faster than SipHash on very short keys, weaker on
/// adversarial input.
#[derive(Debug, Clone, Copy)]
pub struct Fnv;

impl Hasher for Fnv {
    type Concrete = fnv::FnvHasher;

    #[inline]
    fn build() -> Self::Concrete {
        fnv::FnvHasher::default()
    }
}
