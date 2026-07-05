//! Compile-time banded LSH index over any [`MinHasher`].
//!
//! The index stores one sorted `Vec<(u64, u32)>` per band, holding
//! `(band_hash, signature_id)` pairs. Query is one binary search per band
//! followed by a run-length collision count over the union of matching
//! signature ids, returning candidates ranked by how many bands they
//! collided in.
//!
//! ```
//! use minhash_rs::prelude::*;
//! use minhash_rs::index::{LshIndex, NoStore, QueryState, Store};
//!
//! let a: MinHash<u64, 128> = (0u64..30).collect();
//! let b: MinHash<u64, 128> = (15u64..45).collect();
//! let c: MinHash<u64, 128> = (200u64..230).collect();
//!
//! let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> =
//!     LshIndex::from_signatures([a, b, c]);
//!
//! let mut state = QueryState::new();
//! let hits = index.top_k(&a, 2, &mut state);
//! assert_eq!(hits[0].0, 0);        // a is closest to itself
//! assert!((hits[0].1 - 1.0).abs() < 1e-9);
//! ```
//!
//! `PERMUTATIONS` and `BANDS` are both const generics, so `PERMUTATIONS % BANDS == 0`
//! and `BANDS >= 1` are checked at compile time by [`MinHasher::band_hashes`].
//! Runtime band counts are not supported.
//!
//! The `S` type parameter is `NoStore` by default, which drops signatures on
//! insert. Switch to `Store` when you want the index to keep sketches so it
//! can drive a Jaccard refinement pass through [`LshIndex::top_k`].

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::hash::Hash as CoreHash;
use core::marker::PhantomData;

use crate::min_hasher::MinHasher;

// ─── Sealed signature-storage marker ───────────────────────────────────────

mod sealed {
    /// Private supertrait sealing [`SigStore`](super::SigStore) so external
    /// crates cannot add new signature-storage strategies.
    pub trait Sealed {}
}

/// Signature-storage strategy for [`LshIndex`].
///
/// Two shipped implementors, [`Store`] and [`NoStore`], control whether the
/// index keeps a `Vec<K>` alongside the band tables. `Store` unlocks
/// [`LshIndex::signature`] and [`LshIndex::top_k`]. `NoStore` drops each
/// signature on insert and leaves refinement to the caller.
pub trait SigStore<K>: sealed::Sealed {
    /// The concrete storage type. `Vec<K>` for [`Store`], the unit type for
    /// [`NoStore`].
    type Storage: Default;

    /// Push a signature into storage, or drop it on the floor.
    #[doc(hidden)]
    fn store(storage: &mut Self::Storage, signature: K);
}

/// Keep every inserted signature in a `Vec<K>` for later Jaccard refinement.
///
/// Adds `PERMUTATIONS * sizeof(Word)` bytes of memory per signature. At 100M
/// entries with `MinHash<u64, 128>` that is ~100 GB, so `Store` is for tests
/// and small examples. Production at scale wants [`NoStore`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Store;

/// Drop every inserted signature after computing its band hashes.
///
/// The index keeps only the band tables. Callers refine with their own data
/// (raw spectra, exact modified cosine, whatever they own). This is the
/// variant every production build at 100M-scale should pick.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoStore;

impl sealed::Sealed for Store {}
impl sealed::Sealed for NoStore {}

impl<K> SigStore<K> for Store {
    type Storage = Vec<K>;

    #[inline]
    fn store(storage: &mut Self::Storage, signature: K) {
        storage.push(signature);
    }
}

impl<K> SigStore<K> for NoStore {
    type Storage = ();

    #[inline]
    fn store(_storage: &mut (), _signature: K) {
        // Drop the signature on the floor.
    }
}

// ─── Candidate and QueryState ──────────────────────────────────────────────

/// One candidate returned by [`LshIndex::candidates`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// Signature id, assigned in insertion order starting at zero.
    pub id: u32,
    /// Number of bands in which this candidate collided with the query.
    /// Higher is more similar. Ranges over `1..=BANDS`.
    pub collision_count: u32,
}

/// Reusable scratch buffer for [`LshIndex::candidates`] queries.
///
/// The buffer holds up to one entry per band collision across every band,
/// plus the final ranked candidate list. Reusing the same [`QueryState`]
/// across queries removes all per-query heap allocation once the buffer
/// reaches its steady-state size.
#[derive(Debug, Default, Clone)]
pub struct QueryState {
    scratch: Vec<u32>,
    candidates: Vec<Candidate>,
}

impl QueryState {
    /// Fresh state with no allocated capacity. Grows on first query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the internal buffers without releasing their capacity, so the
    /// next query reuses the same allocations.
    pub fn clear(&mut self) {
        self.scratch.clear();
        self.candidates.clear();
    }
}

// ─── LshIndex ──────────────────────────────────────────────────────────────

/// A compile-time banded locality-sensitive hashing index over signatures
/// of type `K: MinHasher<PERMUTATIONS, u64>`.
///
/// See the [module docs](self) for the query flow and storage semantics.
pub struct LshIndex<K, const PERMUTATIONS: usize, const BANDS: usize, S = NoStore>
where
    K: MinHasher<PERMUTATIONS, u64>,
    S: SigStore<K>,
{
    band_tables: [Vec<(u64, u32)>; BANDS],
    signatures: <S as SigStore<K>>::Storage,
    len: u32,
    _marker: PhantomData<S>,
}

impl<K, const PERMUTATIONS: usize, const BANDS: usize, S> LshIndex<K, PERMUTATIONS, BANDS, S>
where
    K: MinHasher<PERMUTATIONS, u64>,
    K::Word: CoreHash,
    S: SigStore<K>,
{
    /// Build an index over the given signatures, sorting each band table
    /// once at the end.
    ///
    /// Signature ids are assigned in the iterator's yield order starting
    /// at zero. `PERMUTATIONS` and `BANDS` are checked at compile time by
    /// [`MinHasher::band_hashes`]. The build is `O(n * BANDS + BANDS * n * log n)`
    /// dominated by the per-band sort.
    ///
    /// # Panics
    ///
    /// Panics if the iterator yields more than `u32::MAX` signatures.
    pub fn from_signatures<I: IntoIterator<Item = K>>(signatures: I) -> Self {
        let mut band_tables: [Vec<(u64, u32)>; BANDS] = core::array::from_fn(|_| Vec::new());
        let mut storage = <<S as SigStore<K>>::Storage>::default();
        let mut len: u32 = 0;

        for sig in signatures {
            let id = len;
            let hashes = sig.band_hashes::<BANDS>();
            for (b, &h) in hashes.iter().enumerate() {
                band_tables[b].push((h, id));
            }
            <S as SigStore<K>>::store(&mut storage, sig);
            len = len
                .checked_add(1)
                .expect("LshIndex holds at most u32::MAX signatures");
        }

        for table in &mut band_tables {
            table.sort_unstable_by_key(|&(h, _)| h);
        }

        Self {
            band_tables,
            signatures: storage,
            len,
            _marker: PhantomData,
        }
    }

    /// Number of signatures currently indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// `true` when no signatures have been inserted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Collect the candidates that collide with `query` in at least one band,
    /// ranked by descending collision count then ascending id for a stable
    /// tie-break.
    ///
    /// Uses `state` as scratch to avoid per-query heap allocation. The
    /// returned slice borrows from `state` and is valid until the next call
    /// that mutates `state`.
    pub fn candidates<'state>(
        &self,
        query: &K,
        state: &'state mut QueryState,
    ) -> &'state [Candidate] {
        state.clear();

        let query_hashes = query.band_hashes::<BANDS>();
        for (b, &qh) in query_hashes.iter().enumerate() {
            let table = &self.band_tables[b];
            let start = table.partition_point(|&(h, _)| h < qh);
            let end = start + table[start..].partition_point(|&(h, _)| h == qh);
            for &(_, id) in &table[start..end] {
                state.scratch.push(id);
            }
        }

        state.scratch.sort_unstable();
        let mut i = 0;
        while i < state.scratch.len() {
            let id = state.scratch[i];
            let mut count = 0u32;
            while i < state.scratch.len() && state.scratch[i] == id {
                count += 1;
                i += 1;
            }
            state.candidates.push(Candidate {
                id,
                collision_count: count,
            });
        }

        state
            .candidates
            .sort_unstable_by(|a, b| match b.collision_count.cmp(&a.collision_count) {
                Ordering::Equal => a.id.cmp(&b.id),
                other => other,
            });

        &state.candidates
    }
}

// ─── Store-only methods ────────────────────────────────────────────────────

impl<K, const PERMUTATIONS: usize, const BANDS: usize> LshIndex<K, PERMUTATIONS, BANDS, Store>
where
    K: MinHasher<PERMUTATIONS, u64>,
    K::Word: CoreHash,
{
    /// Return the stored signature for `id`.
    ///
    /// Only available under [`Store`]. Panics if `id >= self.len()`.
    #[must_use]
    pub fn signature(&self, id: u32) -> &K {
        &self.signatures[id as usize]
    }

    /// Rank the LSH candidates by exact [`MinHasher::estimate_jaccard_index`]
    /// against the stored signatures and return the top `k` as
    /// `(signature_id, jaccard_estimate)` pairs, best first.
    ///
    /// The full candidate set is scored to Jaccard before truncation, which
    /// is the classical LSH refine step. For very long candidate lists,
    /// callers who want to bound the refine cost should call
    /// [`Self::candidates`] instead and cap the number of refined entries
    /// themselves.
    pub fn top_k(&self, query: &K, k: usize, state: &mut QueryState) -> Vec<(u32, f64)> {
        let cands = self.candidates(query, state);
        let mut scored: Vec<(u32, f64)> = cands
            .iter()
            .map(|c| {
                let sig = &self.signatures[c.id as usize];
                (c.id, query.estimate_jaccard_index(sig))
            })
            .collect();
        scored.sort_unstable_by(|a, b| match b.1.partial_cmp(&a.1) {
            Some(Ordering::Equal) | None => a.0.cmp(&b.0),
            Some(other) => other,
        });
        scored.truncate(k);
        scored
    }
}
