//! Tests for the `MinHasher` trait default `estimate_jaccard_index` method.
//!
//! Exercises the mutual-to-dense path with every concrete pairing that shares
//! `Word=u64, Hash=u64, Hasher=SipHashes13, Value=u64`.

extern crate alloc;

use std::collections::HashSet;

use minhash_rs::prelude::*;

/// Build two `MinHash<u64, 128>` sketches from the given value slices and
/// return the sketches and the ground-truth Jaccard index.
fn build_dense_pair(
    a_values: &[u64],
    b_values: &[u64],
) -> (MinHash<u64, 128>, MinHash<u64, 128>, f64) {
    let mut a = MinHash::<u64, 128>::new();
    let mut b = MinHash::<u64, 128>::new();
    for &v in a_values {
        a.insert(v);
    }
    for &v in b_values {
        b.insert(v);
    }
    let a_set: HashSet<u64> = a_values.iter().copied().collect();
    let b_set: HashSet<u64> = b_values.iter().copied().collect();
    let intersection = a_set.intersection(&b_set).count() as f64;
    let union = a_set.union(&b_set).count() as f64;
    let truth = intersection / union;
    (a, b, truth)
}

/// Build two `SparseHashes<u64, 128>` sketches from the given value slices.
fn build_sparse_hashes_pair(
    a_values: &[u64],
    b_values: &[u64],
) -> (SparseHashes<u64, 128>, SparseHashes<u64, 128>) {
    let mut a = SparseHashes::<u64, 128>::new();
    let mut b = SparseHashes::<u64, 128>::new();
    for &v in a_values {
        a.insert(v);
    }
    for &v in b_values {
        b.insert(v);
    }
    (a, b)
}

/// Build a `SparseValues<128>` sketch from the given value slice.
fn build_sparse_values(values: &[u64]) -> SparseValues<128> {
    let mut sv = SparseValues::<128>::new();
    for &v in values {
        sv.insert(v);
    }
    sv
}

#[test]
fn minhash_dense_dense_trait_matches_inherent() {
    let (a, b, _truth) = build_dense_pair(
        &(0..100).collect::<alloc::vec::Vec<_>>(),
        &(50..150).collect::<alloc::vec::Vec<_>>(),
    );
    let trait_result = <MinHash<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&a, &b);
    let inherent_result = a.estimate_jaccard_index(&b);
    assert!(
        (trait_result - inherent_result).abs() < 1e-15,
        "trait path must be bit-identical to inherent path for dense-dense"
    );
}

#[test]
fn sparse_hashes_sparse_sparse_trait_via_dense_oracle() {
    let a_values: alloc::vec::Vec<u64> = (0..100).collect();
    let b_values: alloc::vec::Vec<u64> = (50..150).collect();
    let (sa, sb) = build_sparse_hashes_pair(&a_values, &b_values);
    assert!(sa.is_sparse(), "left sketch must remain sparse");
    assert!(sb.is_sparse(), "right sketch must remain sparse");

    let trait_result =
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&sa, &sb);

    let da: MinHash<u64, 128> = sa.into();
    let db: MinHash<u64, 128> = sb.into();
    let dense_oracle = da.estimate_jaccard_index(&db);

    // Sparse-to-dense promotion is bit-identical to from-scratch MinHash,
    // so the trait path (which calls to_dense) must match the dense oracle.
    assert!(
        (trait_result - dense_oracle).abs() < 1e-15,
        "trait path must match dense oracle after promotion"
    );
}

#[test]
fn sparse_hashes_densified_trait_via_dense_oracle() {
    let a_values: alloc::vec::Vec<u64> = (0..100).collect();
    let b_values: alloc::vec::Vec<u64> = (50..150).collect();
    let (mut sa, mut sb) = build_sparse_hashes_pair(&a_values, &b_values);
    sa.densify();
    sb.densify();
    assert!(sa.is_dense(), "left must be dense after densify");
    assert!(sb.is_dense(), "right must be dense after densify");

    let trait_result =
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&sa, &sb);
    let da: MinHash<u64, 128> = sa.into();
    let db: MinHash<u64, 128> = sb.into();
    let dense_oracle = da.estimate_jaccard_index(&db);

    assert!(
        (trait_result - dense_oracle).abs() < 1e-15,
        "densified trait path must match dense oracle"
    );
}

#[test]
fn cross_wrapper_sparse_hashes_vs_sparse_values() {
    let a_values: alloc::vec::Vec<u64> = (0..100).collect();
    let b_values: alloc::vec::Vec<u64> = (50..150).collect();
    let (sh, _) = build_sparse_hashes_pair(&a_values, &b_values);
    let sv = build_sparse_values(&b_values);

    let trait_result =
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&sh, &sv);

    let da: MinHash<u64, 128> = sh.into();
    let db: MinHash<u64, 128> = sv.into();
    let dense_oracle = da.estimate_jaccard_index(&db);

    assert!(
        (trait_result - dense_oracle).abs() < 1e-15,
        "cross-wrapper trait path must match dense oracle"
    );
}

#[test]
fn cross_wrapper_sparse_values_vs_minhash() {
    let a_values: alloc::vec::Vec<u64> = (0..100).collect();
    let b_values: alloc::vec::Vec<u64> = (50..150).collect();
    let sv = build_sparse_values(&a_values);
    let (mh, _, _) = build_dense_pair(&b_values, &a_values);

    let trait_result = <SparseValues<128> as MinHasher<128, u64>>::estimate_jaccard_index(&sv, &mh);

    let da: MinHash<u64, 128> = sv.into();
    let dense_oracle = da.estimate_jaccard_index(&mh);

    assert!(
        (trait_result - dense_oracle).abs() < 1e-15,
        "SparseValues vs MinHash trait path must match dense oracle"
    );
}
