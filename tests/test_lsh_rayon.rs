//! Equivalence pin between [`LshIndex::from_signatures`] and
//! [`LshIndex::from_signatures_par`].
//!
//! The parallel build shuffles work across rayon threads but must
//! preserve the same id assignment (input order) and end up with the
//! same per-band sorted tables. Any divergence would break query
//! results downstream, so pin it here across representative
//! configurations.

#![cfg(feature = "rayon")]

use minhash_rs::index::{LshIndex, NoStore, Store};
use minhash_rs::prelude::*;

fn deterministic_signature<const P: usize>(seed: u64) -> MinHash<u64, P> {
    // Cheap deterministic pseudo-random input via the crate's own
    // splitmix, so we do not need `rand` in dev-deps.
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut out = MinHash::<u64, P>::new();
    for _ in 0..50 {
        state = state.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
        out.insert(state);
    }
    out
}

fn signatures<const P: usize>(count: usize) -> Vec<MinHash<u64, P>> {
    (0..count as u64)
        .map(deterministic_signature::<P>)
        .collect()
}

fn assert_indices_equal<
    const P: usize,
    const B: usize,
    S: minhash_rs::index::SigStore<MinHash<u64, P>>,
>(
    seq: &LshIndex<MinHash<u64, P>, P, B, S>,
    par: &LshIndex<MinHash<u64, P>, P, B, S>,
) {
    assert_eq!(seq.len(), par.len(), "index lengths must match");
    let mut state = QueryState::new();
    let mut state_par = QueryState::new();
    // For every signature we inserted, both indices must return the
    // exact same candidate order (id and collision count).
    for id in 0..seq.len() {
        // Rebuild the same input to use as a query.
        let query = deterministic_signature::<P>(id as u64);
        let a = seq.candidates(&query, &mut state).to_vec();
        let b = par.candidates(&query, &mut state_par).to_vec();
        assert_eq!(a, b, "candidate slices must match for query id {id}");
    }
}

#[test]
fn from_signatures_par_matches_sequential_p128_b16_nostore() {
    let sigs = signatures::<128>(64);
    let seq: LshIndex<_, 128, 16, NoStore> = LshIndex::from_signatures(sigs.clone());
    let par: LshIndex<_, 128, 16, NoStore> = LshIndex::from_signatures_par(sigs);
    assert_indices_equal(&seq, &par);
}

#[test]
fn from_signatures_par_matches_sequential_p128_b16_store() {
    let sigs = signatures::<128>(64);
    let seq: LshIndex<_, 128, 16, Store> = LshIndex::from_signatures(sigs.clone());
    let par: LshIndex<_, 128, 16, Store> = LshIndex::from_signatures_par(sigs);
    assert_indices_equal(&seq, &par);
}

#[test]
fn from_signatures_par_matches_sequential_p64_b8_nostore() {
    let sigs = signatures::<64>(200);
    let seq: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures(sigs.clone());
    let par: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures_par(sigs);
    assert_indices_equal(&seq, &par);
}

#[test]
fn from_signatures_par_matches_sequential_empty() {
    let sigs: Vec<MinHash<u64, 64>> = Vec::new();
    let seq: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures(sigs.clone());
    let par: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures_par(sigs);
    assert_indices_equal(&seq, &par);
}

#[test]
fn from_signatures_par_matches_sequential_single() {
    let sigs = signatures::<64>(1);
    let seq: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures(sigs.clone());
    let par: LshIndex<_, 64, 8, NoStore> = LshIndex::from_signatures_par(sigs);
    assert_indices_equal(&seq, &par);
}
