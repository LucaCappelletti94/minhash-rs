//! Tests for the `SparseHashes` wrapper.
//!
//! Focus on the state-equivalence invariant (promoted `SparseHashes` matches
//! a from-scratch `MinHash` on the same input), sparse-mode exactness of
//! membership and Jaccard, and the `MinHasher<P>` trait surface.

extern crate alloc;

use minhash_rs::prelude::*;

#[test]
fn new_is_sparse_and_empty_reports_correctly() {
    let sketch = SparseHashes::<u64, 128>::new();
    assert!(sketch.is_sparse());
    assert!(!sketch.is_dense());
    assert_eq!(sketch.count(), 0);
}

#[test]
fn sparse_insert_returns_outcome() {
    let mut sketch = SparseHashes::<u64, 128>::new();
    assert_eq!(sketch.insert(42u64), Outcome::Inserted);
    assert_eq!(sketch.insert(42u64), Outcome::Duplicate);
    assert!(sketch.may_contain(42u64));
    assert!(!sketch.may_contain(99u64));
}

#[test]
fn sparse_promotes_on_overflow() {
    let mut sketch = SparseHashes::<u64, 8>::new();
    let mut promotions = 0;
    for v in 0u64..64 {
        if sketch.insert(v) == Outcome::Promoted {
            promotions += 1;
        }
    }
    assert_eq!(promotions, 1, "exactly one insert triggers promotion");
    assert!(sketch.is_dense());
}

#[test]
fn promoted_state_equals_native_dense() {
    let values: alloc::vec::Vec<u64> = (0u64..2000).collect();
    let mut sparse = SparseHashes::<u64, 32>::new();
    for &v in &values {
        sparse.insert(v);
    }
    let promoted: MinHash<u64, 32> = sparse.into();

    let mut dense = MinHash::<u64, 32>::new();
    for &v in &values {
        dense.insert(v);
    }

    assert_eq!(promoted.as_words(), dense.as_words());
}

#[test]
fn sparse_sparse_jaccard_is_exact() {
    let a: alloc::vec::Vec<u64> = (0u64..40).collect();
    let b: alloc::vec::Vec<u64> = (20u64..60).collect();
    let mut sa = SparseHashes::<u64, 128>::new();
    let mut sb = SparseHashes::<u64, 128>::new();
    for &v in &a {
        sa.insert(v);
    }
    for &v in &b {
        sb.insert(v);
    }
    let j = sa.estimate_jaccard_index(&sb);
    let truth = 20.0 / 60.0;
    assert!((j - truth).abs() < 1e-9, "expected {truth}, got {j}");
}

#[test]
fn minhasher_trait_impl_agrees_with_inherent() {
    let mut sparse = SparseHashes::<u64, 128>::new();
    for v in 0u64..30 {
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::insert(&mut sparse, v);
    }
    for v in 0u64..30 {
        assert!(<SparseHashes<u64, 128> as MinHasher<128, u64>>::may_contain(&sparse, v,));
    }
    let dense = <SparseHashes<u64, 128> as MinHasher<128, u64>>::to_dense(&sparse);
    for v in 0u64..30 {
        assert!(dense.may_contain(v));
    }
}

#[test]
fn minhasher_trait_may_contain_rejects_absent_values() {
    // Defends the `may_contain -> true` mutant on the trait impl: a value
    // never inserted must not be reported present.
    let mut sparse = SparseHashes::<u64, 128>::new();
    for v in 0u64..30 {
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::insert(&mut sparse, v);
    }
    for v in 200u64..230 {
        assert!(
            !<SparseHashes<u64, 128> as MinHasher<128, u64>>::may_contain(&sparse, v),
            "value {v} was never inserted, may_contain must return false"
        );
    }
}

#[test]
fn minhasher_trait_densify_flips_mode() {
    // Defends the `densify with ()` (no-op) mutant on the trait impl: after
    // calling trait densify, the sketch must be in dense mode.
    let mut sparse = SparseHashes::<u64, 128>::new();
    for v in 0u64..30 {
        sparse.insert(v);
    }
    assert!(sparse.is_sparse());
    <SparseHashes<u64, 128> as MinHasher<128, u64>>::densify(&mut sparse);
    assert!(sparse.is_dense());
    assert!(!sparse.is_sparse());
}

#[test]
fn debug_output_is_nonempty_and_names_type() {
    // Defends the `Debug::fmt -> Ok(Default::default())` mutant.
    let sketch = SparseHashes::<u64, 128>::new();
    let rendered = format!("{sketch:?}");
    assert!(!rendered.is_empty());
    assert!(
        rendered.contains("SparseHashes"),
        "Debug output should name the type, got {rendered:?}"
    );
}
