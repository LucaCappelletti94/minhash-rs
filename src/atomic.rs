//! Hash generation as an iterator, and lock-free atomic insertion.
//!
//! Two thin abstractions live in this module:
//! - [`IterHashes`] exposes the per-permutation hash stream as an
//!   [`Iterator`], useful for external consumers that want to plug MinHash's
//!   hash pipeline into another storage layout.
//! - [`AsAtomic`] reinterprets a MinHash's word storage as a slice of
//!   atomics, and [`AtomicFetchInsert`] provides concurrent inserts on top.

use core::hash::{Hash as CoreHash, Hasher as CoreHasher};
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::minhash::MinHash;
use crate::primitive::Primitive;

/// Iterator that emits `count` per-permutation hashes seeded by the raw
/// digest of `value` under `hasher`.
///
/// Zero is never emitted. XorShift has zero as a fixed point, so a stream
/// that ever hits zero would stay there for the rest of the sketch. The
/// guard on the `Hash` value keeps the SplitMix+XorShift stream lively, and
/// a second guard on the truncated `Word` prevents the sparse mode flag
/// (`words[0] == 0`) from being accidentally set by a dense stream whose low
/// bits happen to be zero.
struct HashStream<Word, Hash>
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
    fn new(digest: Hash, count: usize) -> Self {
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

/// An atomic integer that supports atomic minimum-update.
pub trait AtomicFetchMin {
    /// The non-atomic word type stored in this atomic.
    type Word;

    /// Update the value to `min(current, value)` atomically, using `ordering`
    /// for successful compare-exchange stores.
    fn set_min(&self, value: Self::Word, ordering: Ordering);
}

/// Exposes the per-permutation MinHash hash stream as an [`Iterator`].
///
/// Blanket-implemented for every [`MinHash`], parametrised by the same
/// hasher and hash type. External callers get the iterator via
/// [`MinHash::<...>::iter_hashes_from_value`] and its family of specialised
/// entry points.
pub trait IterHashes<Word, const PERMUTATIONS: usize>
where
    Word: Copy + PartialEq,
{
    /// The hash width for the emitted stream.
    type HashType: HashType + Primitive<Word>;

    /// Iterate hashes for `value` using `hasher`.
    fn iter_hashes_from_value<V, HS>(value: V, mut hasher: HS) -> impl Iterator<Item = Word>
    where
        V: CoreHash,
        HS: CoreHasher,
    {
        value.hash(&mut hasher);
        let digest = Self::HashType::from_u64_digest(hasher.finish());
        HashStream::<Word, Self::HashType>::new(digest, PERMUTATIONS)
    }

    /// Iterate hashes for `value` using the SipHash-1-3 default keys.
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// assert!(!minhash.may_contain(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    fn iter_siphashes13_from_value<V: CoreHash>(value: V) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, siphasher::sip128::SipHasher13::new())
    }

    /// Iterate hashes for `value` using FNV-1a with the default seed.
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128, Fnv>::new();
    /// assert!(!minhash.may_contain(42));
    /// minhash.insert(42);
    /// assert!(minhash.may_contain(42));
    /// ```
    fn iter_fnv_from_value<V: CoreHash>(value: V) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, fnv::FnvHasher::default())
    }
}

impl<Word, const PERMUTATIONS: usize, H, Hash> IterHashes<Word, PERMUTATIONS>
    for MinHash<Word, PERMUTATIONS, H, Hash>
where
    Word: Copy + PartialEq,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
{
    type HashType = Hash;
}

/// Reinterpret a MinHash's word storage as a shareable slice of atomics.
///
/// If the sketch is in sparse mode it is densified first, so the atomic view
/// always operates on a valid MinHash signature.
///
/// # Soundness
/// The atomic view is derived from an exclusive `&mut self` borrow, which
/// gives the resulting reference read-write provenance (this is the crucial
/// difference from reinterpreting a shared `&[Word]`, which only grants
/// read-only provenance and is UB to write through). The returned
/// `&[AtomicWord]` is `Sync` and can therefore be shared across threads
/// while the exclusive borrow holds. See Rust issue #76314 for the parallel
/// pattern used by unstable `Atomic*::from_mut_slice`.
///
/// The hasher used for concurrent atomic inserts on the returned slice must
/// match the phantom [`Hasher`] on the sketch and the [`HashType`] chosen
/// for its stream. Using a mismatched hasher or hash width produces
/// meaningless membership results and Jaccard estimates.
pub trait AsAtomic {
    /// The atomic word type backing this MinHash.
    type AtomicWord: AtomicFetchMin;

    /// Reinterpret the MinHash words as a shareable slice of atomics.
    fn as_atomic(&mut self) -> &[Self::AtomicWord];
}

// Emit atomic implementations for one word width, gated on the target
// actually having an atomic of that width. Targets without (say) 64-bit
// atomics simply do not get the u64 atomic API, while narrower widths keep
// working. The `$sparse` parameter is either `check` (u64/usize can be
// sparse, densify first) or `skip` (narrow words are never sparse).
macro_rules! atomic_impls {
    ($word:ty, $atomic:ident, $has:literal, check) => {
        #[cfg(target_has_atomic = $has)]
        impl AtomicFetchMin for core::sync::atomic::$atomic {
            type Word = $word;

            fn set_min(&self, value: Self::Word, ordering: Ordering) {
                let mut current = self.load(Ordering::Relaxed);
                while value < current {
                    match self.compare_exchange_weak(current, value, ordering, Ordering::Relaxed) {
                        Ok(_) => break,
                        Err(observed) => current = observed,
                    }
                }
            }
        }

        #[cfg(target_has_atomic = $has)]
        impl<const PERMUTATIONS: usize, H, Hash> AsAtomic for MinHash<$word, PERMUTATIONS, H, Hash>
        where
            H: Hasher,
            Hash: HashType + Primitive<$word>,
            $word: Primitive<Hash>,
        {
            type AtomicWord = core::sync::atomic::$atomic;

            fn as_atomic(&mut self) -> &[core::sync::atomic::$atomic] {
                if self.is_sparse() {
                    self.densify();
                }
                let words: &mut [$word] = self.as_mut();
                // SAFETY: the atomic has identical size and alignment to
                // `$word`, so the slice layout (data pointer and length) is
                // preserved. The atomic view is derived from a unique `&mut`
                // borrow that is held for the returned slice's lifetime,
                // ruling out concurrent non-atomic access.
                unsafe {
                    core::mem::transmute::<&mut [$word], &[core::sync::atomic::$atomic]>(words)
                }
            }
        }
    };

    ($word:ty, $atomic:ident, $has:literal, skip) => {
        #[cfg(target_has_atomic = $has)]
        impl AtomicFetchMin for core::sync::atomic::$atomic {
            type Word = $word;

            fn set_min(&self, value: Self::Word, ordering: Ordering) {
                let mut current = self.load(Ordering::Relaxed);
                while value < current {
                    match self.compare_exchange_weak(current, value, ordering, Ordering::Relaxed) {
                        Ok(_) => break,
                        Err(observed) => current = observed,
                    }
                }
            }
        }

        #[cfg(target_has_atomic = $has)]
        impl<const PERMUTATIONS: usize, H, Hash> AsAtomic for MinHash<$word, PERMUTATIONS, H, Hash>
        where
            H: Hasher,
            Hash: HashType + Primitive<$word>,
        {
            type AtomicWord = core::sync::atomic::$atomic;

            fn as_atomic(&mut self) -> &[core::sync::atomic::$atomic] {
                // Narrow word types cannot be sparse (no `SparseFor` impl
                // for them), so no densification check needed.
                let words: &mut [$word] = self.as_mut();
                // SAFETY: identical size and alignment; exclusive `&mut`
                // borrow is held for the returned slice's lifetime.
                unsafe {
                    core::mem::transmute::<&mut [$word], &[core::sync::atomic::$atomic]>(words)
                }
            }
        }
    };
}

atomic_impls!(u8, AtomicU8, "8", skip);
atomic_impls!(u16, AtomicU16, "16", skip);
atomic_impls!(u32, AtomicU32, "32", skip);
atomic_impls!(u64, AtomicU64, "64", check);
atomic_impls!(usize, AtomicUsize, "ptr", check);

/// Concurrent insertion into a slice of atomic MinHash words.
///
/// Obtain the slice via [`AsAtomic::as_atomic`], then share it across
/// threads and call one of the specialised inserts. Membership and Jaccard
/// on the original [`MinHash`] happen after the atomic borrow is released.
///
/// The hasher and hash-width used for concurrent inserts must match the
/// phantom [`Hasher`] and [`HashType`] on the sketch. Passing a mismatched
/// pair produces meaningless membership results and Jaccard estimates. The
/// type parameters on each method make the pairing explicit at the call
/// site.
///
/// # Examples
///
/// ```
/// use core::sync::atomic::Ordering;
/// use minhash_rs::prelude::*;
///
/// let mut minhash = MinHash::<u64, 4>::new();
/// {
///     let atomic = minhash.as_atomic();
///     atomic.fetch_insert_with_siphashes13::<_, u64>(42u64, Ordering::Relaxed);
///     atomic.fetch_insert_with_siphashes13::<_, u64>(47u64, Ordering::Relaxed);
/// }
/// assert!(!minhash.is_empty());
/// assert!(minhash.may_contain(42u64));
/// assert!(minhash.may_contain(47u64));
/// ```
pub trait AtomicFetchInsert {
    /// The word type stored in these atomics.
    type Word;

    /// Insert atomically using SipHash-1-3 with default keys as the raw
    /// hasher and `Hash` as the internal hash stream width.
    fn fetch_insert_with_siphashes13<V, Hash>(&self, value: V, ordering: Ordering)
    where
        V: CoreHash,
        Hash: HashType + Primitive<Self::Word>;

    /// Insert atomically using FNV-1a with the default seed as the raw
    /// hasher and `Hash` as the internal hash stream width.
    fn fetch_insert_with_fnv<V, Hash>(&self, value: V, ordering: Ordering)
    where
        V: CoreHash,
        Hash: HashType + Primitive<Self::Word>;
}

impl<A> AtomicFetchInsert for [A]
where
    A: AtomicFetchMin,
    A::Word: Copy + PartialEq,
{
    type Word = A::Word;

    fn fetch_insert_with_siphashes13<V, Hash>(&self, value: V, ordering: Ordering)
    where
        V: CoreHash,
        Hash: HashType + Primitive<Self::Word>,
    {
        let mut hasher = siphasher::sip128::SipHasher13::new();
        value.hash(&mut hasher);
        let digest = Hash::from_u64_digest(hasher.finish());
        for (word, hash) in self
            .iter()
            .zip(HashStream::<A::Word, Hash>::new(digest, self.len()))
        {
            word.set_min(hash, ordering);
        }
    }

    fn fetch_insert_with_fnv<V, Hash>(&self, value: V, ordering: Ordering)
    where
        V: CoreHash,
        Hash: HashType + Primitive<Self::Word>,
    {
        let mut hasher = fnv::FnvHasher::default();
        value.hash(&mut hasher);
        let digest = Hash::from_u64_digest(hasher.finish());
        for (word, hash) in self
            .iter()
            .zip(HashStream::<A::Word, Hash>::new(digest, self.len()))
        {
            word.set_min(hash, ordering);
        }
    }
}
