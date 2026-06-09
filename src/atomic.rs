//! Hash generation and lock-free atomic insertion for MinHash sketches.

use core::hash::{Hash, Hasher};
use core::sync::atomic::Ordering;
use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize};

use fnv::FnvHasher;
use siphasher::sip128::SipHasher13;

use crate::prelude::*;

/// Generate `count` MinHash word hashes from `value` using the provided `hasher`.
///
/// This is shared by both the non-atomic [`IterHashes`] machinery and the
/// atomic insertion path so that the two stay byte-for-byte compatible.
///
/// Zero is never emitted. XorShift has zero as a fixed point, so a hash that
/// ever reaches zero would stay zero for the rest of the stream. In particular,
/// when the seed truncates to zero (about one value in 256 for an 8-bit word)
/// the whole stream would collapse to zero and saturate the sketch from a
/// single insertion. Remapping zero to one keeps the stream non-degenerate for
/// every word width.
fn iter_word_hashes<Word, H, HS>(
    value: H,
    mut hasher: HS,
    count: usize,
) -> impl Iterator<Item = Word>
where
    Word: XorShift + Copy + PartialEq,
    u64: Primitive<Word>,
    H: Hash,
    HS: Hasher,
{
    let zero: Word = 0u64.convert();
    let one: Word = 1u64.convert();

    // Calculate the hash.
    value.hash(&mut hasher);
    let mut hash: Word = hasher.finish().splitmix().splitmix().convert();
    if hash == zero {
        hash = one;
    }

    // Iterate over the words, never emitting the degenerate zero state. The
    // native generators (u8/u16/u32/u64) are bijections on the non-zero space,
    // so once the seed is non-zero they never produce zero; this in-loop guard
    // therefore only fires for widths whose generator truncates a wider type
    // (for example `usize` on a 32-bit target), keeping the stream safe there.
    (0..count).map(move |_| {
        hash = hash.xorshift();
        if hash == zero {
            hash = one;
        }
        hash
    })
}

/// An atomic integer that supports an atomic minimum-update.
pub trait AtomicFetchMin {
    /// The non-atomic word type stored in this atomic.
    type Word;

    /// Set the minimum value atomically
    ///
    /// # Arguments
    /// * `value` - The value to set.
    /// * `ordering` - The ordering to use.
    ///
    fn set_min(&self, value: Self::Word, ordering: Ordering);
}

impl AtomicFetchMin for AtomicU8 {
    type Word = u8;

    fn set_min(&self, value: Self::Word, ordering: Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU16 {
    type Word = u16;

    fn set_min(&self, value: Self::Word, ordering: Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU32 {
    type Word = u32;

    fn set_min(&self, value: Self::Word, ordering: Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicU64 {
    type Word = u64;

    fn set_min(&self, value: Self::Word, ordering: Ordering) {
        self.fetch_min(value, ordering);
    }
}

impl AtomicFetchMin for AtomicUsize {
    type Word = usize;

    fn set_min(&self, value: Self::Word, ordering: Ordering) {
        self.fetch_min(value, ordering);
    }
}

/// Generates the per-permutation hash streams a MinHash uses for a value.
pub trait IterHashes<Word, const PERMUTATIONS: usize>
where
    Word: Min + XorShift + Copy + Eq,
    u64: Primitive<Word>,
{
    /// Iterate on the hashes from the provided value and hasher.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    fn iter_hashes_from_value<H: Hash, HS: Hasher>(
        value: H,
        hasher: HS,
    ) -> impl Iterator<Item = Word> {
        iter_word_hashes(value, hasher, PERMUTATIONS)
    }

    /// Iterate on the SipHasher13 hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    ///
    /// # Examples
    ///
    /// ```rust
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
    fn iter_siphashes13_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, SipHasher13::new())
    }

    /// Iterate on the keyed SipHasher13 hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    /// * `key0` - The first key.
    /// * `key1` - The second key.
    ///
    /// # Examples
    ///
    /// ```rust
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
    fn iter_keyed_siphashes13_from_value<H: Hash>(
        value: H,
        key0: u64,
        key1: u64,
    ) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, SipHasher13::new_with_keys(key0, key1))
    }

    /// Iterate on the FNV hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    ///
    /// # Examples
    ///
    /// ```rust
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
    fn iter_fnv_from_value<H: Hash>(value: H) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, FnvHasher::default())
    }

    /// Iterate on the keyed FNV hashes from the provided value.
    ///
    /// # Arguments
    /// * `value` - The value to hash.
    /// * `key` - The key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minhash_rs::prelude::*;
    ///
    /// let mut minhash = MinHash::<u64, 128>::new();
    /// let key = 0x0123456789ABCDEF;
    ///
    /// assert!(!minhash.may_contain_value_with_keyed_fnv(42, key));
    /// minhash.insert_with_keyed_fnv(42, key);
    /// assert!(minhash.may_contain_value_with_keyed_fnv(42, key));
    /// ```
    fn iter_keyed_fnv_from_value<H: Hash>(value: H, key: u64) -> impl Iterator<Item = Word> {
        Self::iter_hashes_from_value(value, FnvHasher::with_key(key))
    }
}

impl<Word: Min + XorShift + Copy + Eq, const PERMUTATIONS: usize> IterHashes<Word, PERMUTATIONS>
    for MinHash<Word, PERMUTATIONS>
where
    u64: Primitive<Word>,
{
}

/// Reinterpret the words of a [`MinHash`] as a slice of atomics.
///
/// # Soundness
/// The atomic view is derived from an exclusive `&mut self` borrow, which gives
/// the resulting reference read-write provenance (this is the crucial
/// difference from reinterpreting a shared `&[Word]`, which only grants
/// read-only provenance and is undefined behavior to write through). The
/// returned `&[AtomicWord]` can then be shared across threads (it is `Sync`) so
/// that values can be inserted concurrently with [`AtomicFetchInsert`]. The
/// exclusive borrow is held for the lifetime of the returned slice, so no
/// non-atomic access to the same words can race with the atomic inserts.
///
/// This mirrors what the (still unstable) `Atomic*::from_mut_slice` helpers do
/// internally; see Rust issue #76314.
pub trait AsAtomic {
    /// The atomic word type backing this MinHash.
    type AtomicWord: AtomicFetchMin;

    /// Reinterpret the MinHash words as a shareable slice of atomics.
    fn as_atomic(&mut self) -> &[Self::AtomicWord];
}

macro_rules! impl_as_atomic {
    ($word:ty, $atomic:ty) => {
        impl<const PERMUTATIONS: usize> AsAtomic for MinHash<$word, PERMUTATIONS> {
            type AtomicWord = $atomic;

            fn as_atomic(&mut self) -> &[$atomic] {
                let words: &mut [$word] = self.as_mut();
                // SAFETY: `$atomic` has the same size and alignment as `$word`,
                // so the slice layout (data pointer and length) is identical.
                // The atomic view is derived from a unique `&mut` borrow, so it
                // carries read-write provenance, and that exclusive borrow is
                // held for the lifetime of the returned slice, ruling out
                // concurrent non-atomic access to the same memory.
                unsafe { core::mem::transmute::<&mut [$word], &[$atomic]>(words) }
            }
        }
    };
}

impl_as_atomic!(u8, AtomicU8);
impl_as_atomic!(u16, AtomicU16);
impl_as_atomic!(u32, AtomicU32);
impl_as_atomic!(u64, AtomicU64);
impl_as_atomic!(usize, AtomicUsize);

/// Concurrent insertion into a slice of atomic MinHash words.
///
/// Obtain the slice from [`AsAtomic::as_atomic`], then share it across threads
/// and call these methods. Membership and Jaccard estimation are performed on
/// the original [`MinHash`] once the atomic borrow has been released.
///
/// # Examples
///
/// ```
/// use minhash_rs::prelude::*;
/// use core::sync::atomic::Ordering;
///
/// let mut minhash = MinHash::<u64, 4>::new();
/// {
///     let atomic = minhash.as_atomic();
///     atomic.fetch_insert_with_siphashes13(42, Ordering::Relaxed);
///     atomic.fetch_insert_with_siphashes13(47, Ordering::Relaxed);
/// }
/// assert!(!minhash.is_empty());
/// assert!(minhash.may_contain_value_with_siphashes13(42));
/// assert!(minhash.may_contain_value_with_siphashes13(47));
/// ```
pub trait AtomicFetchInsert {
    /// The (non-atomic) word type stored in each atomic.
    type Word;

    /// Insert a value atomically using the SipHasher13.
    fn fetch_insert_with_siphashes13<H: Hash>(&self, value: H, ordering: Ordering);

    /// Insert a value atomically using the keyed SipHasher13.
    fn fetch_insert_with_keyed_siphashes13<H: Hash>(
        &self,
        value: H,
        key0: u64,
        key1: u64,
        ordering: Ordering,
    );

    /// Insert a value atomically using the FNV hash.
    fn fetch_insert_with_fnv<H: Hash>(&self, value: H, ordering: Ordering);

    /// Insert a value atomically using the keyed FNV hash.
    fn fetch_insert_with_keyed_fnv<H: Hash>(&self, value: H, key: u64, ordering: Ordering);
}

impl<A> AtomicFetchInsert for [A]
where
    A: AtomicFetchMin,
    A::Word: XorShift + Copy + PartialEq,
    u64: Primitive<A::Word>,
{
    type Word = A::Word;

    fn fetch_insert_with_siphashes13<H: Hash>(&self, value: H, ordering: Ordering) {
        for (word, hash) in self
            .iter()
            .zip(iter_word_hashes(value, SipHasher13::new(), self.len()))
        {
            word.set_min(hash, ordering);
        }
    }

    fn fetch_insert_with_keyed_siphashes13<H: Hash>(
        &self,
        value: H,
        key0: u64,
        key1: u64,
        ordering: Ordering,
    ) {
        for (word, hash) in self.iter().zip(iter_word_hashes(
            value,
            SipHasher13::new_with_keys(key0, key1),
            self.len(),
        )) {
            word.set_min(hash, ordering);
        }
    }

    fn fetch_insert_with_fnv<H: Hash>(&self, value: H, ordering: Ordering) {
        for (word, hash) in
            self.iter()
                .zip(iter_word_hashes(value, FnvHasher::default(), self.len()))
        {
            word.set_min(hash, ordering);
        }
    }

    fn fetch_insert_with_keyed_fnv<H: Hash>(&self, value: H, key: u64, ordering: Ordering) {
        for (word, hash) in self.iter().zip(iter_word_hashes(
            value,
            FnvHasher::with_key(key),
            self.len(),
        )) {
            word.set_min(hash, ordering);
        }
    }
}
