//! Tests for the `SparseValues` codec-generic sparse-value wrapper.

extern crate alloc;

use dsi_bitstream::dispatch::{code_consts, CodeLen, DynamicCodeRead, DynamicCodeWrite};
use minhash_rs::prelude::*;
use sketching_core::sparse_value_list::ConstCode;

type Gamma = ConstCode<{ code_consts::GAMMA }>;
type Delta = ConstCode<{ code_consts::DELTA }>;

#[test]
fn new_is_sparse_and_empty() {
    let sketch = SparseValues::<128>::new();
    assert!(sketch.is_sparse());
    assert!(!sketch.is_dense());
    assert_eq!(sketch.count(), 0);
}

#[test]
fn sparse_insert_returns_outcome() {
    let mut sketch = SparseValues::<128>::new();
    assert_eq!(sketch.insert(42u64), Outcome::Inserted);
    assert_eq!(sketch.insert(42u64), Outcome::Duplicate);
    assert!(sketch.may_contain(42u64));
    assert!(!sketch.may_contain(99u64));
}

fn assert_state_equivalence<const P: usize, Code>(values: &[u64])
where
    Code: DynamicCodeRead + DynamicCodeWrite + CodeLen + Copy,
{
    let mut sparse = SparseValues::<P, SipHashes13, u64, Code>::new();
    for &v in values {
        sparse.insert(v);
    }
    let promoted: MinHash<u64, P> = sparse.into();

    let mut dense = MinHash::<u64, P>::new();
    for &v in values {
        dense.insert(v);
    }

    assert_eq!(promoted.as_words(), dense.as_words());
}

#[test]
fn state_equivalence_gamma() {
    assert_state_equivalence::<128, Gamma>(&[]);
    assert_state_equivalence::<128, Gamma>(&[42]);
    assert_state_equivalence::<128, Gamma>(&[1, 2, 3, 4, 5]);
    let values: alloc::vec::Vec<u64> = (0u64..2048).collect();
    assert_state_equivalence::<32, Gamma>(&values);
}

#[test]
fn state_equivalence_delta() {
    let values: alloc::vec::Vec<u64> = (100u64..200).collect();
    assert_state_equivalence::<64, Delta>(&values);
    assert_state_equivalence::<64, Gamma>(&values);
}

#[test]
fn different_codecs_produce_same_promoted_state() {
    let values: alloc::vec::Vec<u64> = (0u64..100).collect();
    let mut sg = SparseValues::<64, SipHashes13, u64, Gamma>::new();
    let mut sd = SparseValues::<64, SipHashes13, u64, Delta>::new();
    for &v in &values {
        sg.insert(v);
        sd.insert(v);
    }
    let dg: MinHash<u64, 64> = sg.into();
    let dd: MinHash<u64, 64> = sd.into();
    assert_eq!(dg.as_words(), dd.as_words());
}

#[test]
fn sparse_sparse_jaccard_is_exact() {
    let a: alloc::vec::Vec<u64> = (0u64..40).collect();
    let b: alloc::vec::Vec<u64> = (20u64..60).collect();
    let mut sa = SparseValues::<128>::new();
    let mut sb = SparseValues::<128>::new();
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
fn gamma_promotion_boundary_for_p8() {
    // Available bit budget: 64 * 7 - 9 = 439 bits. `sparse_value_list`
    // encodes the raw gap between consecutive stored values (not `gap - 1`
    // as the older hyperloglog-rs value list did), so consecutive integers
    // cost `gamma_len(1) = 3` bits each after the first. Total cost for
    // storing [0..N) descending: `gamma_len(N-1) + 3 * (N-1)`. Largest
    // solution to `gamma_len(N-1) + 3 * (N-1) <= 439` is:
    //   N = 142: `gamma_len(141) + 3 * 141 = 15 + 423 = 438`. Fits.
    //   N = 143: `gamma_len(142) + 3 * 142 = 17 + 426 = 443`. Does not fit.
    // So `N* = 142`.
    let mut sketch = SparseValues::<8, SipHashes13, u64, Gamma>::new();
    let mut last_sparse_count: u32 = 0;
    for v in 0u64..1024 {
        sketch.insert(v);
        if sketch.is_dense() {
            break;
        }
        last_sparse_count = sketch.count();
    }
    assert_eq!(last_sparse_count, 142);
}

/// Red test for the `MinHasher` trait-method state-equivalence gap.
///
/// Insertion via the `MinHasher<P>` trait method on `SparseValues` MUST
/// produce, after promotion, a signature bit-identical to what the same
/// trait method on `MinHash` produces on the same input. If it does not, the
/// two wrappers are not interchangeable behind `T: MinHasher<P>`, and any
/// generic code (including cross-wrapper Jaccard via the trait default) is
/// silently comparing signatures over different transformations.
#[test]
fn trait_insert_state_equivalent_to_minhash_on_u64() {
    let values: alloc::vec::Vec<u64> = (0u64..30).collect();

    let mut sparse: SparseValues<128> = SparseValues::new();
    let mut dense: MinHash<u64, 128> = MinHash::new();
    for &v in &values {
        <SparseValues<128> as MinHasher<128, u64>>::insert(&mut sparse, v);
        <MinHash<u64, 128> as MinHasher<128, u64>>::insert(&mut dense, v);
    }

    let promoted: MinHash<u64, 128> = sparse.into();
    assert_eq!(
        promoted.as_words(),
        dense.as_words(),
        "MinHasher::insert on SparseValues MUST produce a state bit-identical \
         to MinHasher::insert on MinHash for the same input sequence"
    );
}
#[test]
#[allow(clippy::clone_on_copy)]
fn debug_clone_default_and_from_conversion_all_run() {
    let mut sv = SparseValues::<128>::new();
    for v in 0u64..100 {
        sv.insert(v);
    }

    // Debug: assert non-empty and contains struct name
    let debug_str = format!("{sv:?}");
    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("SparseValues"));

    // Clone: assert count is preserved
    let cloned = sv.clone();
    assert_eq!(cloned.count(), sv.count());

    // Default: assert sparse and empty
    let default_sv = SparseValues::<128>::default();
    assert!(default_sv.is_sparse());
    assert_eq!(default_sv.count(), 0);

    // From<SparseValues> for MinHash: lift and verify
    let dense: MinHash<u64, 128> = sv.into();
    assert!(dense.may_contain(42u64));
    assert!(dense.may_contain(99u64));
    assert!(!dense.may_contain(1000u64));
}

#[test]
fn metadata_reports_expected_values() {
    let sv = SparseValues::<128>::new();
    assert_eq!(sv.number_of_permutations(), 128);
    assert_eq!(sv.memory(), 128 * 64);
    assert!(sv.is_sparse());
    assert!(!sv.is_dense());

    // Insert enough values to force promotion to dense mode.
    // P=128 tail is 127 * 8 = 1016 bytes; 4096 consecutive values overflow gamma.
    let mut sv2 = SparseValues::<128>::new();
    for v in 0u64..4096 {
        sv2.insert(v);
    }
    assert!(sv2.is_dense());
    assert!(!sv2.is_sparse());
}

#[test]
fn may_contain_dense_fallback_matches_minhash() {
    // Build a sketch and manually densify to exercise the dense fallback path
    // of may_contain (the else branch that delegates to self.inner.may_contain).
    let mut sv = SparseValues::<128>::new();
    let values: alloc::vec::Vec<u64> = (0u64..200).collect();
    for &v in &values {
        sv.insert(v);
    }
    sv.densify();
    assert!(sv.is_dense());

    // Build the oracle MinHash on the same input
    let mut oracle = MinHash::<u64, 128>::new();
    for &v in &values {
        oracle.insert(v);
    }

    // Test a mix of positive and negative values
    let positives: alloc::vec::Vec<u64> = (0u64..200).step_by(20).collect();
    let negatives = [1000u64, 2000u64, 99999u64, 0u64.wrapping_sub(1)];

    for v in &positives {
        assert!(
            sv.may_contain(*v) == oracle.may_contain(*v),
            "may_contain mismatch for value {v}"
        );
    }
    for v in &negatives {
        assert!(
            sv.may_contain(*v) == oracle.may_contain(*v),
            "may_contain mismatch for value {v}"
        );
    }
}

#[test]
fn into_minhash_consumes_and_densifies() {
    let mut sv = SparseValues::<128>::new();
    let values: alloc::vec::Vec<u64> = (0u64..64).collect();
    for &v in &values {
        sv.insert(v);
    }

    // Still sparse before conversion
    assert!(sv.is_sparse());

    // into_minhash consumes and produces a dense MinHash
    let mh = sv.into_minhash();
    assert!(mh.is_full());
    for &v in &values {
        assert!(
            mh.may_contain(v),
            "promoted MinHash must contain every inserted value"
        );
    }
}

#[test]
fn estimate_jaccard_index_mixed_modes_and_all_sparse() {
    // Build two sketches with partial overlap.
    let set_a: alloc::vec::Vec<u64> = (0u64..200).collect();
    let set_b: alloc::vec::Vec<u64> = (100u64..300).collect();

    // sa stays sparse (fewer values fit in sparse budget)
    let mut sa = SparseValues::<64>::new();
    for &v in &set_a {
        sa.insert(v);
    }

    // sb gets promoted to dense (more values overflow sparse budget)
    let mut sb = SparseValues::<64>::new();
    for &v in &set_b {
        sb.insert(v);
    }

    // Mixed-mode Jaccard via SparseValues
    let mixed_jaccard = sa.estimate_jaccard_index(&sb);

    // Dense-vs-dense oracle
    let mut oracle_a = MinHash::<u64, 64>::new();
    let mut oracle_b = MinHash::<u64, 64>::new();
    for &v in &set_a {
        oracle_a.insert(v);
    }
    for &v in &set_b {
        oracle_b.insert(v);
    }
    let oracle_jaccard = oracle_a.estimate_jaccard_index(&oracle_b);

    assert!(
        (mixed_jaccard - oracle_jaccard).abs() < 0.15,
        "mixed-mode Jaccard {mixed_jaccard} diverges from dense oracle {oracle_jaccard}"
    );

    // Two identical sparse sketches yield 1.0
    let set_c: alloc::vec::Vec<u64> = (0u64..20).collect();
    let mut sc1 = SparseValues::<128>::new();
    let mut sc2 = SparseValues::<128>::new();
    for &v in &set_c {
        sc1.insert(v);
        sc2.insert(v);
    }
    assert!(sc1.is_sparse() && sc2.is_sparse());
    let identical_jaccard = sc1.estimate_jaccard_index(&sc2);
    assert!(
        (identical_jaccard - 1.0).abs() < 1e-9,
        "identical sparse sketches should yield Jaccard 1.0, got {identical_jaccard}"
    );
}

#[test]
fn minhasher_trait_impl_dispatches_to_inherent() {
    let values: alloc::vec::Vec<u64> = (0u64..100).collect();

    // Trait-driven sketch
    let mut trait_sv: SparseValues<128> = SparseValues::new();
    for &v in &values {
        <SparseValues<128> as MinHasher<128, u64>>::insert(&mut trait_sv, v);
    }

    // Inherent-method sketch on the same input
    let mut inherent_sv: SparseValues<128> = SparseValues::new();
    for &v in &values {
        inherent_sv.insert(v);
    }

    // may_contain via trait matches inherent
    for &v in &values {
        assert_eq!(
            <SparseValues<128> as MinHasher<128, u64>>::may_contain(&trait_sv, v),
            inherent_sv.may_contain(v),
            "trait may_contain must match inherent for value {v}"
        );
    }

    // densify via trait produces same dense state as inherent
    <SparseValues<128> as MinHasher<128, u64>>::densify(&mut trait_sv);
    inherent_sv.densify();
    let trait_dense = <SparseValues<128> as MinHasher<128, u64>>::to_dense(&trait_sv);
    let inherent_dense = inherent_sv.into_minhash();
    assert_eq!(
        trait_dense.as_words(),
        inherent_dense.as_words(),
        "trait densify must produce bit-identical state to inherent densify"
    );

    // to_dense via trait matches into_minhash on a fresh copy
    let mut fresh_sv: SparseValues<128> = SparseValues::new();
    for &v in &values {
        fresh_sv.insert(v);
    }
    let trait_dense = <SparseValues<128> as MinHasher<128, u64>>::to_dense(&fresh_sv);
    let inherent_dense = fresh_sv.into_minhash();
    assert_eq!(
        trait_dense.as_words(),
        inherent_dense.as_words(),
        "trait to_dense must match into_minhash"
    );
}
