//! Pin the LSH refine claim in the README: when the sketch type feeding
//! [`LshIndex`] keeps its operands in sparse mode, `top_k` invokes the
//! exact sparse-sparse Jaccard fast path, so the returned scores equal
//! the true set Jaccard within the underlying hasher's collision floor
//! rather than the classical `Θ(J(1 - J) / PERMUTATIONS)`
//! dense-estimator variance.
//!
//! Primitives are already pinned by `test_sparse_hashes.rs`,
//! `test_sparse_values.rs`, and `test_min_hasher.rs`. This test closes
//! the loop end-to-end through the LSH pipeline (`from_signatures` +
//! `top_k`) so that the crate-level claim in `README.md` is exercised
//! by a real integration test.

extern crate alloc;

use alloc::collections::BTreeSet;

use minhash_rs::index::{LshIndex, Store};
use minhash_rs::prelude::*;

fn set_of(range: core::ops::Range<u64>) -> BTreeSet<u64> {
    range.collect()
}

fn sparse_hashes<const P: usize>(range: core::ops::Range<u64>) -> SparseHashes<u64, P> {
    let mut s = SparseHashes::<u64, P>::new();
    for v in range {
        s.insert(v);
    }
    s
}

fn sparse_values<const P: usize>(range: core::ops::Range<u64>) -> SparseValues<P> {
    let mut s = SparseValues::<P>::new();
    for v in range {
        s.insert(v);
    }
    s
}

fn true_jaccard(a: &BTreeSet<u64>, b: &BTreeSet<u64>) -> f64 {
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// Two heavily overlapping sparse ranges will band-collide, so both
/// end up in the candidate list. `top_k` must then score them via
/// `MinHasher::estimate_jaccard_index`, which for sparse-sparse
/// operands is the exact set intersection over the retained digests.
/// The score of the self-match must be exactly 1.0, and the score of
/// the near neighbour must match the true set Jaccard within the
/// SipHash-1-3 collision floor of about `1 / 2^64`.
#[test]
fn top_k_sparse_hashes_returns_exact_score_on_self_and_neighbour() {
    const P: usize = 128;
    const BANDS: usize = 16;

    let a_range = 0u64..30;
    let b_range = 10u64..40; // overlaps a on {10..30}, |a ∩ b| = 20, |a ∪ b| = 40, J = 0.5.

    let a_set = set_of(a_range.clone());
    let b_set = set_of(b_range.clone());

    let sig_a = sparse_hashes::<P>(a_range.clone());
    let sig_b = sparse_hashes::<P>(b_range);
    assert!(sig_a.is_sparse() && sig_b.is_sparse());

    let query = sparse_hashes::<P>(a_range);
    assert!(query.is_sparse());

    let index: LshIndex<_, P, BANDS, Store> = LshIndex::from_signatures([sig_a, sig_b]);
    let mut state = QueryState::new();
    let hits = index.top_k(&query, 2, &mut state);

    assert!(
        !hits.is_empty(),
        "query should retrieve at least the self match"
    );
    assert_eq!(
        hits[0].0, 0,
        "signature 0 (the query itself) is the top hit"
    );
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(
            hits[0].1, 1.0,
            "sparse-sparse refine must produce Jaccard exactly 1.0 on self-match, got {}",
            hits[0].1
        );
    }

    // Match against b, if it was retrieved by candidate generation.
    if let Some(&(_, jab)) = hits.iter().find(|(id, _)| *id == 1) {
        let expected = true_jaccard(&a_set, &b_set);
        assert!(
            (jab - expected).abs() < 1e-12,
            "sparse-sparse refine on a vs b must be exact within the hash-collision floor, \
             got {jab}, expected {expected}"
        );
    }
}

/// Same claim on `SparseValues`. Sparse-value operands are exact on
/// the original input elements with no hash-collision floor, so the
/// score must equal the true set Jaccard bit-for-bit.
#[test]
fn top_k_sparse_values_returns_bit_exact_score_on_self_and_neighbour() {
    const P: usize = 128;
    const BANDS: usize = 16;

    let a_range = 0u64..30;
    let b_range = 10u64..40;

    let a_set = set_of(a_range.clone());
    let b_set = set_of(b_range.clone());

    let sig_a = sparse_values::<P>(a_range.clone());
    let sig_b = sparse_values::<P>(b_range);
    assert!(sig_a.is_sparse() && sig_b.is_sparse());

    let query = sparse_values::<P>(a_range);
    assert!(query.is_sparse());

    let index: LshIndex<_, P, BANDS, Store> = LshIndex::from_signatures([sig_a, sig_b]);
    let mut state = QueryState::new();
    let hits = index.top_k(&query, 2, &mut state);

    assert_ne!(hits, [] as [(u32, f64); 0]);
    assert_eq!(hits[0].0, 0);
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(
            hits[0].1, 1.0,
            "sparse-value refine must produce Jaccard exactly 1.0 on self-match, got {}",
            hits[0].1
        );
    }

    if let Some(&(_, jab)) = hits.iter().find(|(id, _)| *id == 1) {
        let expected = true_jaccard(&a_set, &b_set);
        #[allow(clippy::float_cmp)]
        {
            assert_eq!(
                jab, expected,
                "sparse-value refine is exact on input elements, expected {expected}, got {jab}"
            );
        }
    }
}

/// Regression: after either operand densifies, `top_k` falls back to
/// the standard dense estimator. Score is no longer bit-exact but the
/// self match still ranks first.
#[test]
fn top_k_after_densification_still_ranks_self_match_first() {
    const P: usize = 128;
    const BANDS: usize = 16;

    let sig_a: SparseHashes<u64, P> = sparse_hashes(0u64..30);
    let sig_b: SparseHashes<u64, P> = sparse_hashes(10u64..40);

    let query_dense: SparseHashes<u64, P> = {
        let mut s = sparse_hashes(0u64..30);
        s.densify();
        s
    };
    assert!(query_dense.is_dense());

    let index: LshIndex<_, P, BANDS, Store> = LshIndex::from_signatures([sig_a, sig_b]);
    let mut state = QueryState::new();
    let hits = index.top_k(&query_dense, 2, &mut state);
    assert_ne!(hits, [] as [(u32, f64); 0]);
    assert_eq!(hits[0].0, 0);
}
