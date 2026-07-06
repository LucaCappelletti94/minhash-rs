//! Equivalence pin for the batched `FromIterator` paths on `MinHash`,
//! `SparseHashes`, and `SparseValues`.
//!
//! For each sketch type and a range of input sizes, build two sketches from
//! the same deterministic random input: one via the streaming `insert` loop
//! (reference), one via `FromIterator::from_iter` (batched). Assert that the
//! resulting word arrays are identical.
//!
//! This catches any regression in the batched build primitives that would
//! cause them to diverge from the streaming path.

extern crate alloc;

use alloc::vec::Vec;
use minhash_rs::hashtype::HashType;
use minhash_rs::prelude::*;

/// Generate deterministic pseudo-random u64 values.
fn gen_values(count: usize, seed: u64) -> Vec<u64> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state.splitmix();
            state.xorshift()
        })
        .collect()
}

/// Input sizes that stress the batched code paths: empty, single, small,
/// sparse overflow boundary at P-1 and P+1, CHUNK boundary at 512, and large.
const SIZES: [usize; 10] = [0, 1, 3, 63, 64, 65, 511, 512, 513, 10_000];

// ─── MinHash ────────────────────────────────────────────────────────────────

fn minhash_equivalence<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);
        let batched: MinHash<u64, P> = values.iter().copied().collect();

        let mut reference = MinHash::<u64, P>::new();
        for &v in &values {
            reference.insert(v);
        }

        assert_eq!(
            batched.as_words(),
            reference.as_words(),
            "MinHash<u64, {P}>: batched from_iter must match insert loop at size {size}"
        );
    }
}

#[test]
fn minhash_p64_from_iter_matches_insert_loop() {
    minhash_equivalence::<64>();
}

#[test]
fn minhash_p128_from_iter_matches_insert_loop() {
    minhash_equivalence::<128>();
}

/// Narrow-`Word` config: `Word = u32`, `Hash = u64`. Exercises the
/// non-lossless narrow path in the batched build: `Primitive<u32>` on
/// `u64` returns `IS_LOSSLESS = false`, so `maybe_guard` keeps its word
/// guard, and a narrowing xorshift step can produce a zero low half of a
/// nonzero digest that the guard has to lift back to `one`.
fn minhash_narrow_word_equivalence<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);
        let batched: MinHash<u32, P, SipHashes13, u64> = values.iter().copied().collect();

        let mut reference = MinHash::<u32, P, SipHashes13, u64>::new();
        for &v in &values {
            reference.insert(v);
        }

        assert_eq!(
            batched.as_words(),
            reference.as_words(),
            "MinHash<u32, {P}, SipHashes13, u64>: batched from_iter must match insert loop at size {size}"
        );
    }
}

#[test]
fn minhash_narrow_word_p64_from_iter_matches_insert_loop() {
    minhash_narrow_word_equivalence::<64>();
}

#[test]
fn minhash_narrow_word_p128_from_iter_matches_insert_loop() {
    minhash_narrow_word_equivalence::<128>();
}

// ─── SparseHashes ───────────────────────────────────────────────────────────

fn sparse_hashes_equivalence<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);
        let batched: SparseHashes<u64, P> = values.iter().copied().collect();

        let mut reference = SparseHashes::<u64, P>::new();
        for &v in &values {
            reference.insert(v);
        }

        let batched_dense: MinHash<u64, P> = MinHash::from(batched);
        let reference_dense: MinHash<u64, P> = MinHash::from(reference);
        assert_eq!(
            batched_dense.as_words(),
            reference_dense.as_words(),
            "SparseHashes<u64, {P}>: batched from_iter must match insert loop at size {size}"
        );
    }
}

#[test]
fn sparse_hashes_p64_from_iter_matches_insert_loop() {
    sparse_hashes_equivalence::<64>();
}

#[test]
fn sparse_hashes_p128_from_iter_matches_insert_loop() {
    sparse_hashes_equivalence::<128>();
}

// ─── SparseValues ───────────────────────────────────────────────────────────

fn sparse_values_equivalence<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);
        let batched: SparseValues<P> = values.iter().copied().collect();

        let mut reference = SparseValues::<P>::new();
        for &v in &values {
            reference.insert(v);
        }

        let batched_dense: MinHash<u64, P> = MinHash::from(batched);
        let reference_dense: MinHash<u64, P> = MinHash::from(reference);
        assert_eq!(
            batched_dense.as_words(),
            reference_dense.as_words(),
            "SparseValues<{P}>: batched from_iter must match insert loop at size {size}"
        );
    }
}

#[test]
fn sparse_values_p64_from_iter_matches_insert_loop() {
    sparse_values_equivalence::<64>();
}

#[test]
fn sparse_values_p128_from_iter_matches_insert_loop() {
    sparse_values_equivalence::<128>();
}

// ─── From against MinHash::from_iter ───────────────────────────────────────

fn sparse_hashes_from_matches_minhash_from_iter<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);

        let sparse: SparseHashes<u64, P> = values.iter().copied().collect();
        let from_sparse: MinHash<u64, P> = MinHash::from(sparse);

        let direct: MinHash<u64, P> = values.iter().copied().collect();
        assert_eq!(
            from_sparse.as_words(),
            direct.as_words(),
            "MinHash::from(SparseHashes<u64, {P}>) must equal MinHash::from_iter at size {size}"
        );
    }
}

#[test]
fn sparse_hashes_p64_from_equals_minhash_from_iter() {
    sparse_hashes_from_matches_minhash_from_iter::<64>();
}

#[test]
fn sparse_hashes_p128_from_equals_minhash_from_iter() {
    sparse_hashes_from_matches_minhash_from_iter::<128>();
}

fn sparse_values_from_matches_minhash_from_iter<const P: usize>() {
    for &size in &SIZES {
        let values = gen_values(size, 42);

        let sparse: SparseValues<P> = values.iter().copied().collect();
        let from_sparse: MinHash<u64, P> = MinHash::from(sparse);

        let direct: MinHash<u64, P> = values.iter().copied().collect();
        assert_eq!(
            from_sparse.as_words(),
            direct.as_words(),
            "MinHash::from(SparseValues<{P}>) must equal MinHash::from_iter at size {size}"
        );
    }
}

#[test]
fn sparse_values_p64_from_equals_minhash_from_iter() {
    sparse_values_from_matches_minhash_from_iter::<64>();
}

#[test]
fn sparse_values_p128_from_equals_minhash_from_iter() {
    sparse_values_from_matches_minhash_from_iter::<128>();
}
