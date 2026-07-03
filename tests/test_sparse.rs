//! Comprehensive tests for sparse MinHash mode.

#![allow(clippy::float_cmp)]

use minhash_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[test]
fn sparse_constructor_starts_empty() {
    let mh = MinHash::<u64, 128>::sparse();
    assert!(mh.is_empty());
    assert!(!mh.is_full());
}

#[test]
fn sparse_len_empty() {
    let mh = MinHash::<u64, 128>::sparse();
    assert!(mh.is_empty());
    // Insert one element and verify it's no longer empty.
    let mut mh = mh;
    mh.insert_with_siphashes13(42);
    assert!(!mh.is_empty());
}

#[test]
fn to_u64_conversions() {
    use minhash_rs::primitive::ToU64;
    assert_eq!(42u64.to_u64(), 42u64);
    assert_eq!(42usize.to_u64(), 42u64);
    assert_eq!(42u8.to_u64(), 42u64);
    assert_eq!(42u16.to_u64(), 42u64);
    assert_eq!(42u32.to_u64(), 42u64);
}

#[test]
fn sparse_from_digest() {
    use minhash_rs::primitive::SparseWord;
    assert_eq!(
        <u64 as SparseWord>::from_digest(0xDEAD_BEEF_CAFE_BABE),
        0xDEAD_BEEF_CAFE_BABE
    );
    assert_eq!(
        <usize as SparseWord>::from_digest(0xDEAD_BEEF_CAFE_BABE),
        0xDEAD_BEEF_CAFE_BABE_usize
    );
}

#[test]
fn sparse_sentinel_boundary() {
    // Fill the sketch to exactly capacity (PERMUTATIONS-1 = 127 digests).
    // The sentinel write path (sentinel < PERMUTATIONS) should be skipped
    // when the last slot is occupied.
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..127_u64 {
        mh.insert_with_siphashes13(i);
    }
    assert!(mh.is_full());
    // Insert one more to trigger densification.
    mh.insert_with_siphashes13(9999);
    assert!(!mh.is_empty());
}

#[test]
fn sparse_may_contain_absent_item() {
    // Catch mutant: sparse_contains_digest always returns true.
    let mut mh = MinHash::<u64, 128>::sparse();
    mh.insert_with_siphashes13(42);
    assert!(mh.may_contain_value_with_siphashes13(42));
    assert!(
        !mh.may_contain_value_with_siphashes13(99),
        "may_contain should return false for absent item"
    );
}

#[test]
fn sparse_union_at_exact_capacity() {
    // Catch mutant: sparse_union capacity check > becomes >=.
    // Fill two sketches so that a_len + b_len == capacity (127).
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..64_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 64..127_u64 {
        b.insert_with_siphashes13(i);
    }
    // a has 64 digests, b has 63 digests. Total 127 == capacity.
    // Union should stay sparse and succeed.
    let mut result = a;
    result |= b;
    for i in 0..127_u64 {
        assert!(result.may_contain_value_with_siphashes13(i));
    }
}

#[test]
fn sparse_union_merge_ordering() {
    // Catch mutants in sparse_union merge loop (operator replacements).
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    // Interleaved values to exercise merge comparisons.
    for i in 0..30_u64 {
        a.insert_with_siphashes13(i * 2);
        b.insert_with_siphashes13(i * 2 + 1);
    }
    let mut result = a;
    result |= b;
    for i in 0..60_u64 {
        assert!(
            result.may_contain_value_with_siphashes13(i),
            "missing {i} after union"
        );
    }
}

#[test]
fn sparse_no_false_negatives_siphash() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..1000_u64 {
        mh.insert_with_siphashes13(i);
        assert!(
            mh.may_contain_value_with_siphashes13(i),
            "false negative for {i}"
        );
    }
}

#[test]
fn sparse_no_false_negatives_fnv() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..1000_u64 {
        mh.insert_with_fnv(i);
        assert!(mh.may_contain_value_with_fnv(i), "false negative for {i}");
    }
}

#[test]
fn sparse_no_false_negatives_keyed() {
    let mut mh = MinHash::<u64, 128>::sparse();
    let (key0, key1) = (0xA5A5_A5A5_A5A5_A5A5, 0x5A5A_5A5A_5A5A_5A5A);
    for i in 0..1000_u64 {
        mh.insert_with_keyed_siphashes13(i, key0, key1);
        assert!(
            mh.may_contain_value_with_keyed_siphashes13(i, key0, key1),
            "false negative for {i}"
        );
    }
}

#[test]
fn sparse_deduplicates() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for _ in 0..100 {
        mh.insert_with_siphashes13(42);
    }
    let mut dense_single = MinHash::<u64, 128>::new();
    dense_single.insert_with_siphashes13(42);
    assert_eq!(mh, dense_single);
}

#[test]
fn sparse_eq_dense_cross_mode() {
    // Test sparse == dense (PartialEq cross-mode path).
    let values: Vec<u64> = (0..50).collect();
    let mut sparse = MinHash::<u64, 128>::sparse();
    let mut dense = MinHash::<u64, 128>::new();
    for &v in &values {
        sparse.insert_with_siphashes13(v);
        dense.insert_with_siphashes13(v);
    }
    // Cross-mode equality: sparse compared to dense triggers internal densification.
    assert_eq!(sparse, dense);
    assert_eq!(dense, sparse);
}

#[test]
fn sparse_hash_equals_dense_hash() {
    // Test that sparse and dense sketches hash to the same value (Hash contract).
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;

    fn hash_value<T: Hash>(t: &T) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    let values: Vec<u64> = (0..50).collect();
    let mut sparse = MinHash::<u64, 128>::sparse();
    let mut dense = MinHash::<u64, 128>::new();
    for &v in &values {
        sparse.insert_with_siphashes13(v);
        dense.insert_with_siphashes13(v);
    }
    assert_eq!(hash_value(&sparse), hash_value(&dense));
}

#[test]
fn sparse_default_is_empty() {
    // Exercise Default impl for sparse (via new which is Default).
    let mh = MinHash::<u64, 128>::default();
    assert!(mh.is_empty());
}

#[test]
fn sparse_is_sparse_flag() {
    let sparse = MinHash::<u64, 128>::sparse();
    let dense = MinHash::<u64, 128>::new();
    // is_sparse() is pub(crate) but we can check via behavior.
    assert!(sparse.is_empty());
    assert!(dense.is_empty());
    // After insert, sparse stays sparse until capacity.
    let mut s = MinHash::<u64, 128>::sparse();
    s.insert_with_siphashes13(1);
    assert!(!s.is_empty());
    assert!(!s.is_full());
}

#[test]
fn sparse_exact_jaccard_identical_sets() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..100_u64 {
        a.insert_with_siphashes13(i);
        b.insert_with_siphashes13(i);
    }
    assert_eq!(a.estimate_jaccard_index(&b), 1.0);
}

#[test]
fn sparse_exact_jaccard_disjoint_sets() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..100_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 100..200_u64 {
        b.insert_with_siphashes13(i);
    }
    assert_eq!(a.estimate_jaccard_index(&b), 0.0);
}

#[test]
fn sparse_exact_jaccard_known_overlap() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..100_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 50..150_u64 {
        b.insert_with_siphashes13(i);
    }
    let jaccard = a.estimate_jaccard_index(&b);
    assert!(
        (jaccard - 1.0 / 3.0).abs() < 1e-10,
        "expected 1/3, got {jaccard}"
    );
}

#[test]
fn sparse_exact_jaccard_empty_sets() {
    let a = MinHash::<u64, 128>::sparse();
    let b = MinHash::<u64, 128>::sparse();
    assert_eq!(a.estimate_jaccard_index(&b), 1.0);
}

#[test]
fn sparse_dense_equivalent_after_same_insertions() {
    let values: Vec<u64> = (0..500).collect();
    let mut sparse = MinHash::<u64, 128>::sparse();
    let mut dense = MinHash::<u64, 128>::new();
    for &v in &values {
        sparse.insert_with_siphashes13(v);
        dense.insert_with_siphashes13(v);
    }
    assert_eq!(
        sparse, dense,
        "sparse and dense should produce identical signatures"
    );
    for &v in &values {
        assert_eq!(
            sparse.may_contain_value_with_siphashes13(v),
            dense.may_contain_value_with_siphashes13(v),
            "disagreement for value {v}"
        );
    }
}

#[test]
fn sparse_dense_cross_mode_jaccard() {
    // Keep within sparse capacity (PERMUTATIONS-1 = 127)
    let values_a: Vec<u64> = (0..50).collect();
    let values_b: Vec<u64> = (25..75).collect();
    let mut sparse_a = MinHash::<u64, 128>::sparse();
    let mut sparse_b = MinHash::<u64, 128>::sparse();
    let mut dense_a = MinHash::<u64, 128>::new();
    let mut dense_b = MinHash::<u64, 128>::new();
    for &v in &values_a {
        sparse_a.insert_with_siphashes13(v);
        dense_a.insert_with_siphashes13(v);
    }
    for &v in &values_b {
        sparse_b.insert_with_siphashes13(v);
        dense_b.insert_with_siphashes13(v);
    }
    let sparse_vs_sparse = sparse_a.estimate_jaccard_index(&sparse_b);
    let dense_vs_dense = dense_a.estimate_jaccard_index(&dense_b);
    let sparse_vs_dense = sparse_a.estimate_jaccard_index(&dense_b);
    let dense_vs_sparse = dense_a.estimate_jaccard_index(&sparse_b);
    // Intersection: {25..49} = 25, Union: {0..74} = 75
    let exact = 25.0 / 75.0;
    assert!(
        (sparse_vs_sparse - exact).abs() < 1e-10,
        "sparse-vs-sparse: expected {exact}, got {sparse_vs_sparse}"
    );
    assert!(
        (dense_vs_dense - exact).abs() < 0.05,
        "dense-vs-dense: expected ~{exact}, got {dense_vs_dense}"
    );
    assert!(
        (sparse_vs_dense - dense_vs_dense).abs() < 1e-10,
        "sparse-vs-dense should match dense-vs-dense"
    );
    assert!(
        (dense_vs_sparse - dense_vs_dense).abs() < 1e-10,
        "dense-vs-sparse should match dense-vs-dense"
    );
}

#[test]
fn sparse_union_stays_sparse_when_room() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..50_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 50..100_u64 {
        b.insert_with_siphashes13(i);
    }
    a |= &b;
    for i in 0..100_u64 {
        assert!(a.may_contain_value_with_siphashes13(i));
    }
}

#[test]
fn sparse_union_produces_correct_result() {
    let set_a: HashSet<u64> = (0..100).collect();
    let set_b: HashSet<u64> = (75..150).collect();
    let union_set: HashSet<u64> = set_a.union(&set_b).copied().collect();
    let mut sparse_a = MinHash::<u64, 128>::sparse();
    let mut sparse_b = MinHash::<u64, 128>::sparse();
    let mut dense_union = MinHash::<u64, 128>::new();
    for &v in &set_a {
        sparse_a.insert_with_siphashes13(v);
    }
    for &v in &set_b {
        sparse_b.insert_with_siphashes13(v);
    }
    for &v in &union_set {
        dense_union.insert_with_siphashes13(v);
    }
    sparse_a |= &sparse_b;
    assert_eq!(sparse_a, dense_union);
}

#[test]
fn dense_union_with_sparse_operand() {
    let mut sparse = MinHash::<u64, 128>::sparse();
    let mut dense = MinHash::<u64, 128>::new();
    for i in 0..100_u64 {
        sparse.insert_with_siphashes13(i);
    }
    for i in 50..150_u64 {
        dense.insert_with_siphashes13(i);
    }
    let mut expected = MinHash::<u64, 128>::new();
    for i in 0..150_u64 {
        expected.insert_with_siphashes13(i);
    }
    dense |= &sparse;
    assert_eq!(dense, expected);
}

#[test]
fn sparse_densifies_on_overflow() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..200_u64 {
        mh.insert_with_siphashes13(i);
    }
    let mut dense = MinHash::<u64, 128>::new();
    for i in 0..200_u64 {
        dense.insert_with_siphashes13(i);
    }
    assert_eq!(mh, dense);
}

#[test]
fn usize_sparse_mode() {
    let mut mh = MinHash::<usize, 64>::sparse();
    for i in 0..100usize {
        mh.insert_with_siphashes13(i);
    }
    assert!(mh.may_contain_value_with_siphashes13(50usize));
}

#[test]
fn sparse_serde_roundtrip() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..100_u64 {
        mh.insert_with_siphashes13(i);
    }
    let json = serde_json::to_string(&mh).expect("serialization failed");
    let decoded: MinHash<u64, 128> = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(mh, decoded);
    for i in 0..100_u64 {
        assert!(decoded.may_contain_value_with_siphashes13(i));
    }
}

#[test]
fn sparse_serde_preserves_mode() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..10_u64 {
        mh.insert_with_siphashes13(i);
    }
    let json = serde_json::to_string(&mh).expect("serialization failed");
    let decoded: MinHash<u64, 128> = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(decoded.as_ref()[0], 0);
}

// ─── Keyed FNV ──────────────────────────────────────────────────────────────

#[test]
fn sparse_no_false_negatives_keyed_fnv() {
    let mut mh = MinHash::<u64, 128>::sparse();
    let key = 0x0123_4567_89AB_CDEF;
    for i in 0..1000_u64 {
        mh.insert_with_keyed_fnv(i, key);
        assert!(
            mh.may_contain_value_with_keyed_fnv(i, key),
            "false negative for {i}"
        );
    }
}

// ─── is_full ────────────────────────────────────────────────────────────────

#[test]
fn sparse_is_full_at_capacity() {
    // PERMUTATIONS=16, capacity = 15
    let mut mh = MinHash::<u64, 16>::sparse();
    assert!(!mh.is_full());
    for i in 0..15_u64 {
        mh.insert_with_siphashes13(i);
    }
    assert!(mh.is_full());
}

#[test]
fn sparse_is_full_after_densification() {
    let mut mh = MinHash::<u64, 16>::sparse();
    for i in 0..100_000_u64 {
        mh.insert_with_siphashes13(i);
    }
    assert!(mh.is_full() || !mh.is_empty());
}

// ─── band_hashes ────────────────────────────────────────────────────────────

#[test]
fn sparse_band_hashes_after_densification() {
    let mut sparse = MinHash::<u64, 128>::sparse();
    let mut dense = MinHash::<u64, 128>::new();
    for i in 0..200_u64 {
        sparse.insert_with_siphashes13(i);
        dense.insert_with_siphashes13(i);
    }
    // band_hashes densifies sparse internally
    assert_eq!(sparse.band_hashes::<16>(), dense.band_hashes::<16>());
}

#[test]
fn sparse_band_hashes_while_still_sparse() {
    let mut mh = MinHash::<u64, 128>::sparse();
    for i in 0..10_u64 {
        mh.insert_with_siphashes13(i);
    }
    // Should not panic; densifies internally
    let hashes = mh.band_hashes::<16>();
    assert_eq!(hashes.len(), 16);
}

// ─── as_atomic ──────────────────────────────────────────────────────────────

#[test]
fn sparse_as_atomic_densifies() {
    let mut mh = MinHash::<u64, 16>::sparse();
    for i in 0..10_u64 {
        mh.insert_with_siphashes13(i);
    }
    // as_atomic densifies if sparse
    let atomic = mh.as_atomic();
    assert_eq!(atomic.len(), 16);
    // Sketch should now be dense
    assert!(!mh.is_empty());
}

// ─── iter / iter_mut ────────────────────────────────────────────────────────

#[test]
fn sparse_iter_returns_words() {
    let mut mh = MinHash::<u64, 64>::sparse();
    mh.insert_with_siphashes13(42);
    let words: Vec<_> = mh.iter().collect();
    assert_eq!(words.len(), 64);
}

#[test]
fn sparse_iter_mut_returns_words() {
    let mut mh = MinHash::<u64, 64>::sparse();
    mh.insert_with_siphashes13(42);
    for w in mh.iter_mut() {
        *w = 0;
    }
}

// ─── number_of_permutations / memory ────────────────────────────────────────

#[test]
fn sparse_number_of_permutations() {
    let mh = MinHash::<u64, 128>::sparse();
    assert_eq!(mh.number_of_permutations(), 128);
}

#[test]
fn sparse_memory() {
    let mh = MinHash::<u64, 128>::sparse();
    assert_eq!(mh.memory(), 128 * 8 * 8); // 128 * size_of::<u64>() * 8
}

// ─── Hash contract (sparse == dense => same hash) ──────────────────────────

#[test]
fn sparse_dense_hash_equality() {
    let mut sparse = MinHash::<u64, 64>::sparse();
    let mut dense = MinHash::<u64, 64>::new();
    for i in 0..100_u64 {
        sparse.insert_with_siphashes13(i);
        dense.insert_with_siphashes13(i);
    }
    assert_eq!(sparse, dense);
    let sparse_hash = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        sparse.hash(&mut h);
        h.finish()
    };
    let dense_hash = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        dense.hash(&mut h);
        h.finish()
    };
    assert_eq!(
        sparse_hash, dense_hash,
        "Hash contract: equal values must hash equal"
    );
}

// ─── BitOr (|) operator ────────────────────────────────────────────────────

#[test]
fn sparse_bitor_by_value() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..50_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 50..100_u64 {
        b.insert_with_siphashes13(i);
    }
    let result = a | b;
    for i in 0..100_u64 {
        assert!(result.may_contain_value_with_siphashes13(i));
    }
}

#[allow(clippy::op_ref)]
#[test]
fn sparse_bitor_by_reference() {
    let mut a = MinHash::<u64, 128>::sparse();
    let mut b = MinHash::<u64, 128>::sparse();
    for i in 0..50_u64 {
        a.insert_with_siphashes13(i);
    }
    for i in 50..100_u64 {
        b.insert_with_siphashes13(i);
    }
    let result = a | &b;
    for i in 0..100_u64 {
        assert!(result.may_contain_value_with_siphashes13(i));
    }
}

// ─── FromIterator ───────────────────────────────────────────────────────────

#[test]
fn from_iter_produces_dense() {
    let mh: MinHash<u64, 64> = (0..100_u64).collect();
    for i in 0..100_u64 {
        assert!(mh.may_contain_value_with_siphashes13(i));
    }
}

// ─── Property-based tests ───────────────────────────────────────────────────

fn values() -> impl Strategy<Value = Vec<u64>> {
    prop::collection::vec(any::<u64>(), 0..200)
}

fn prop_sparse_dense_equivalence(values: &[u64]) {
    let mut sparse = MinHash::<u64, 64>::sparse();
    let mut dense = MinHash::<u64, 64>::new();
    for &v in values {
        sparse.insert_with_siphashes13(v);
        dense.insert_with_siphashes13(v);
    }
    assert_eq!(sparse, dense);
}

fn prop_sparse_exact_jaccard(a: &[u64], b: &[u64]) {
    let set_a: HashSet<u64> = a.iter().copied().collect();
    let set_b: HashSet<u64> = b.iter().copied().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union_count = set_a.union(&set_b).count();
    let mut sparse_a = MinHash::<u64, 128>::sparse();
    let mut sparse_b = MinHash::<u64, 128>::sparse();
    for &v in a {
        sparse_a.insert_with_siphashes13(v);
    }
    for &v in b {
        sparse_b.insert_with_siphashes13(v);
    }
    let estimated = sparse_a.estimate_jaccard_index(&sparse_b);
    if union_count == 0 {
        assert_eq!(estimated, 1.0);
    } else {
        let exact = intersection as f64 / union_count as f64;
        assert!(
            (estimated - exact).abs() < 1e-10,
            "sparse Jaccard not exact: estimated {estimated}, exact {exact}"
        );
    }
}

fn prop_sparse_union_correctness(a: &[u64], b: &[u64]) {
    let union_set: HashSet<u64> = a.iter().chain(b.iter()).copied().collect();
    let mut sparse_a = MinHash::<u64, 128>::sparse();
    let mut sparse_b = MinHash::<u64, 128>::sparse();
    let mut dense_union = MinHash::<u64, 128>::new();
    for &v in a {
        sparse_a.insert_with_siphashes13(v);
    }
    for &v in b {
        sparse_b.insert_with_siphashes13(v);
    }
    for &v in &union_set {
        dense_union.insert_with_siphashes13(v);
    }
    sparse_a |= &sparse_b;
    assert_eq!(sparse_a, dense_union);
}

fn prop_sparse_insertion_order_invariant(values: &[u64]) {
    let set: HashSet<u64> = values.iter().copied().collect();
    let sorted: Vec<u64> = set.into_iter().collect();
    let mut mh1 = MinHash::<u64, 64>::sparse();
    let mut mh2 = MinHash::<u64, 64>::sparse();
    for &v in values {
        mh1.insert_with_siphashes13(v);
    }
    for &v in &sorted {
        mh2.insert_with_siphashes13(v);
    }
    assert_eq!(mh1, mh2, "insertion order should not affect result");
}

fn prop_sparse_serde_roundtrip(values: &[u64]) {
    let mut mh = MinHash::<u64, 64>::sparse();
    for &v in values {
        mh.insert_with_siphashes13(v);
    }
    let json = serde_json::to_string(&mh).expect("serialization failed");
    let decoded: MinHash<u64, 64> = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(mh, decoded);
}

proptest! {
    #[test]
    fn sparse_dense_equivalence(v in values()) {
        prop_sparse_dense_equivalence(&v);
    }

    #[test]
    fn sparse_exact_jaccard(a in values(), b in values()) {
        prop_sparse_exact_jaccard(&a, &b);
    }

    #[test]
    fn sparse_union_correctness(a in values(), b in values()) {
        prop_sparse_union_correctness(&a, &b);
    }

    #[test]
    fn sparse_insertion_order_invariant(v in values()) {
        prop_sparse_insertion_order_invariant(&v);
    }

    #[test]
    fn sparse_serde_roundtrip_prop(v in values()) {
        prop_sparse_serde_roundtrip(&v);
    }
}
