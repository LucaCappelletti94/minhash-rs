//! Hasher trait and marker types for compile-time hasher identity.
//!
//! The [`Hasher`] trait is implemented by marker types that identify a hash
//! strategy. Each marker is a zero-sized type (unit struct) so it can be used
//! as a phantom type parameter on [`MinHash`](crate::minhash::MinHash).
//!
//! Sketches built with different hashers are different Rust types, which
//! prevents silent correctness bugs from comparing Jaccard estimates or
//! equality across incompatible hash streams.

/// A hash strategy that can produce a stream of `u64` hashes from a value.
///
/// The hasher always produces `u64` digests regardless of the [`MinHash`] word
/// type. The word type controls storage width only; narrow types (`u8`, `u16`,
/// `u32`) truncate the `u64` hash stream.
pub trait Hasher {
    /// The concrete `core::hash::Hasher` type used by this strategy.
    type Concrete: core::hash::Hasher;

    /// Build a new hasher instance with default configuration.
    fn build() -> Self::Concrete;

    /// Build a hasher with explicit keys.
    ///
    /// Returns `Some` for keyed hasher markers (e.g., [`SipHashes13Keyed`],
    /// [`FnvKeyed`]) and `None` for unkeyed ones.
    #[must_use]
    fn build_with_keys(_keys: &[u64]) -> Option<Self::Concrete> {
        None
    }
}

/// SipHash-1-3 with default keys.
///
/// This is the default hasher for [`MinHash`](crate::minhash::MinHash).
#[derive(Debug, Clone, Copy)]
pub struct SipHashes13;

impl Hasher for SipHashes13 {
    type Concrete = siphasher::sip128::SipHasher13;

    fn build() -> Self::Concrete {
        siphasher::sip128::SipHasher13::new()
    }
}

/// SipHash-1-3 with custom keys provided at construction time.
///
/// The keys are not stored in this type (it is zero-sized). They are passed to
/// [`MinHash::new_with_keys`](crate::minhash::MinHash::new_with_keys) and
/// stored in the [`MinHash`](crate::minhash::MinHash) instance itself.
#[derive(Debug, Clone, Copy)]
pub struct SipHashes13Keyed;

impl Hasher for SipHashes13Keyed {
    type Concrete = siphasher::sip128::SipHasher13;

    fn build() -> Self::Concrete {
        siphasher::sip128::SipHasher13::new()
    }

    fn build_with_keys(keys: &[u64]) -> Option<Self::Concrete> {
        if keys.len() >= 2 {
            Some(siphasher::sip128::SipHasher13::new_with_keys(
                keys[0], keys[1],
            ))
        } else {
            None
        }
    }
}

/// FNV-1a with default configuration (zero seed).
#[derive(Debug, Clone, Copy)]
pub struct Fnv;

impl Hasher for Fnv {
    type Concrete = fnv::FnvHasher;

    fn build() -> Self::Concrete {
        fnv::FnvHasher::default()
    }
}

/// FNV-1a with a custom seed provided at construction time.
///
/// The seed is not stored in this type (it is zero-sized). It is passed to
/// [`MinHash::new_with_keys`](crate::minhash::MinHash::new_with_keys) and
/// stored in the [`MinHash`](crate::minhash::MinHash) instance itself.
#[derive(Debug, Clone, Copy)]
pub struct FnvKeyed;

impl Hasher for FnvKeyed {
    type Concrete = fnv::FnvHasher;

    fn build() -> Self::Concrete {
        fnv::FnvHasher::default()
    }

    fn build_with_keys(keys: &[u64]) -> Option<Self::Concrete> {
        keys.first().copied().map(fnv::FnvHasher::with_key)
    }
}
