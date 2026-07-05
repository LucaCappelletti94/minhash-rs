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
fn sparse_hashes_sparse_sparse_trait_takes_fast_path() {
    // Both operands still sparse. The trait method delegates to the
    // inherent sparse-sparse fast path, which is exact on the retained
    // digest sets. The exact answer matches the true set Jaccard within
    // the 64-bit hash-collision floor (~1e-15). The dense estimator's
    // per-register agreement fraction differs by O(1 / sqrt(P)) in
    // expectation and does not enter this assertion.
    let a_values: alloc::vec::Vec<u64> = (0..100).collect();
    let b_values: alloc::vec::Vec<u64> = (50..150).collect();
    let (sa, sb) = build_sparse_hashes_pair(&a_values, &b_values);
    assert!(sa.is_sparse(), "left sketch must remain sparse");
    assert!(sb.is_sparse(), "right sketch must remain sparse");

    let trait_result =
        <SparseHashes<u64, 128> as MinHasher<128, u64>>::estimate_jaccard_index(&sa, &sb);

    let a_set: alloc::collections::BTreeSet<u64> = a_values.iter().copied().collect();
    let b_set: alloc::collections::BTreeSet<u64> = b_values.iter().copied().collect();
    #[allow(clippy::cast_precision_loss)]
    let truth = a_set.intersection(&b_set).count() as f64 / a_set.union(&b_set).count() as f64;

    assert!(
        (trait_result - truth).abs() < 1e-9,
        "sparse fast path must match true Jaccard within collision floor"
    );

    // Also assert the trait method reaches the same value as the inherent
    // method: they must be bit-identical because the trait body delegates
    // straight to the inherent.
    let inherent_result = sa.estimate_jaccard_index(&sb);
    assert!(
        (trait_result - inherent_result).abs() < 1e-15,
        "trait dispatch must be bit-identical to the inherent method"
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
fn minhasher_trait_may_contain_dispatches_for_minhash() {
    // Defends the `may_contain -> false` mutant on the `MinHasher` impl for
    // `MinHash`: a value that was inserted must be reported present.
    let mut mh = MinHash::<u64, 128>::new();
    let inserted: alloc::vec::Vec<u64> = (0u64..30).collect();
    for &v in &inserted {
        <MinHash<u64, 128> as MinHasher<128, u64>>::insert(&mut mh, v);
    }
    for &v in &inserted {
        assert!(
            <MinHash<u64, 128> as MinHasher<128, u64>>::may_contain(&mh, v),
            "trait may_contain must return true for inserted value {v}"
        );
    }

    // Defends the `may_contain -> true` mutant on the `MinHasher` impl for
    // `MinHash`. On an empty sketch every register sits at the maximal
    // sentinel, so inserting any real value would strictly lower at least
    // one register, which check_hash_stream translates into a proof of
    // absence. If the trait method is mutated to always return true this
    // assertion fires.
    let empty = MinHash::<u64, 128>::new();
    for v in 0u64..64 {
        assert!(
            !<MinHash<u64, 128> as MinHasher<128, u64>>::may_contain(&empty, v),
            "may_contain on an empty sketch must be false for value {v}"
        );
    }
}

#[test]
fn minhash_trait_to_dense_returns_populated_sketch_not_default() {
    // Defends the `<MinHash as MinHasher>::to_dense -> Default::default()`
    // mutant. The trait impl on `MinHash` is the identity (`*self`), so
    // the result must equal the input, and it must not equal the fresh
    // default sketch when the input was populated.
    let mh: MinHash<u64, 128> = (0u64..30).collect();
    let trait_dense = <MinHash<u64, 128> as MinHasher<128, u64>>::to_dense(&mh);
    assert_eq!(
        trait_dense, mh,
        "MinHash trait to_dense must be the identity, not Default::default()"
    );
    let default = MinHash::<u64, 128>::default();
    assert_ne!(
        trait_dense, default,
        "MinHash trait to_dense on a populated sketch must not return the default (empty) sketch"
    );
}
