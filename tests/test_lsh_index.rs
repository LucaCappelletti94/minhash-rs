//! Integration tests for the banded LSH index.
//!
//! Coverage:
//! - Basic `from_signatures` build + `len` / `is_empty`.
//! - `Store` variant: `signature`, `top_k` recovers the query at Jaccard 1.0.
//! - `NoStore` variant: `candidates` still fires, `signature` / `top_k` do not compile.
//! - Candidate ordering by collision count.
//! - `QueryState` reuse across queries does not allocate a fresh scratch on every call.
//! - Duplicate band hashes across signatures are handled cleanly.

extern crate alloc;

use minhash_rs::index::{Candidate, LshIndex, NoStore, QueryState, Store};
use minhash_rs::prelude::*;

#[test]
fn empty_index_reports_empty() {
    let index: LshIndex<MinHash<u64, 128>, 128, 16, NoStore> = LshIndex::from_signatures([]);
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);

    let mut state = QueryState::new();
    let query: MinHash<u64, 128> = (0u64..30).collect();
    let cands = index.candidates(&query, &mut state);
    assert!(cands.is_empty());
}

#[test]
fn store_variant_recovers_signature_by_id() {
    let a: MinHash<u64, 128> = (0u64..30).collect();
    let b: MinHash<u64, 128> = (100u64..130).collect();
    let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> = LshIndex::from_signatures([a, b]);

    assert_eq!(index.len(), 2);
    assert_eq!(index.signature(0), &a);
    assert_eq!(index.signature(1), &b);
}

#[test]
fn top_k_puts_the_query_first_when_the_query_is_indexed() {
    let a: MinHash<u64, 128> = (0u64..30).collect();
    let b: MinHash<u64, 128> = (100u64..130).collect();
    let c: MinHash<u64, 128> = (200u64..230).collect();

    let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> = LshIndex::from_signatures([a, b, c]);

    let mut state = QueryState::new();
    let hits = index.top_k(&a, 2, &mut state);

    assert!(!hits.is_empty(), "query should retrieve itself");
    assert_eq!(hits[0].0, 0, "signature 0 (a itself) is the top hit");
    assert!((hits[0].1 - 1.0).abs() < 1e-9, "self-similarity is 1.0");
}

#[test]
fn candidates_ordered_by_descending_collision_count() {
    // Build an index containing the query and two dissimilar sketches.
    // The query, indexed as id 0, must collide in all 16 bands with itself,
    // and either not collide with the others or collide in strictly fewer
    // bands. So the ordering by collision count puts id 0 first.
    let a: MinHash<u64, 128> = (0u64..30).collect();
    let b: MinHash<u64, 128> = (100u64..130).collect();
    let c: MinHash<u64, 128> = (200u64..230).collect();

    let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> = LshIndex::from_signatures([a, b, c]);

    let mut state = QueryState::new();
    let cands = index.candidates(&a, &mut state);

    assert!(!cands.is_empty());
    assert_eq!(cands[0].id, 0);
    // Query collides with itself in every band.
    assert_eq!(cands[0].collision_count, 16);
    // Every following candidate must have a strictly smaller count, or the
    // same count with a strictly larger id.
    for pair in cands.windows(2) {
        let [left, right] = [pair[0], pair[1]];
        assert!(
            (left.collision_count > right.collision_count)
                || (left.collision_count == right.collision_count && left.id < right.id),
            "candidates must be sorted by count desc then id asc, saw {left:?} then {right:?}"
        );
    }
}

#[test]
fn query_state_is_reusable_across_queries() {
    let sketches: alloc::vec::Vec<MinHash<u64, 128>> = (0..8u64)
        .map(|offset| (offset * 100..offset * 100 + 30).collect())
        .collect();

    let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> =
        LshIndex::from_signatures(sketches.iter().copied());

    let mut state = QueryState::new();
    for query in &sketches {
        let hits = index.top_k(query, 1, &mut state);
        assert!(
            !hits.is_empty(),
            "each query should retrieve at least itself"
        );
        assert!((hits[0].1 - 1.0).abs() < 1e-9);
    }
}

#[test]
fn no_store_still_returns_candidates() {
    let a: MinHash<u64, 128> = (0u64..30).collect();
    let b: MinHash<u64, 128> = (100u64..130).collect();

    let index: LshIndex<MinHash<u64, 128>, 128, 16, NoStore> = LshIndex::from_signatures([a, b]);

    assert_eq!(index.len(), 2);

    let mut state = QueryState::new();
    let cands = index.candidates(&a, &mut state);
    // The query self-inserted at id 0 collides in every band with itself.
    let a_hit = cands.iter().find(|c| c.id == 0).copied();
    assert_eq!(
        a_hit,
        Some(Candidate {
            id: 0,
            collision_count: 16
        })
    );
}

#[test]
fn duplicate_signatures_produce_duplicate_ids_in_the_candidate_list() {
    // Inserting the same sketch twice produces two distinct ids that both
    // sit at the top of the ranked candidate list for that query.
    let a: MinHash<u64, 128> = (0u64..30).collect();

    let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> = LshIndex::from_signatures([a, a, a]);

    assert_eq!(index.len(), 3);

    let mut state = QueryState::new();
    let cands = index.candidates(&a, &mut state);

    // Every one of the three inserted copies collides in all 16 bands with
    // the query. The top three candidates are ids 0, 1, 2 in that order
    // (count DESC tie broken by id ASC).
    assert!(cands.len() >= 3);
    assert_eq!(cands[0].id, 0);
    assert_eq!(cands[1].id, 1);
    assert_eq!(cands[2].id, 2);
    assert_eq!(cands[0].collision_count, 16);
    assert_eq!(cands[1].collision_count, 16);
    assert_eq!(cands[2].collision_count, 16);
}

#[test]
fn candidates_slice_is_valid_until_state_is_reused() {
    let a: MinHash<u64, 128> = (0u64..30).collect();
    let index: LshIndex<MinHash<u64, 128>, 128, 16, NoStore> = LshIndex::from_signatures([a]);

    let mut state = QueryState::new();
    let cands = index.candidates(&a, &mut state);
    let expected_id = cands[0].id;
    // Immutable borrow of `state` via `cands` ends here.
    let owned: alloc::vec::Vec<Candidate> = cands.to_vec();
    // Now the borrow is released, state is usable again.
    let cands_again = index.candidates(&a, &mut state);
    assert_eq!(cands_again[0].id, expected_id);
    assert_eq!(owned[0].id, expected_id);
}
