//! Tests for `MinHashArray`, a fixed-size array of independent MinHash sketches.
// Jaccard estimates against an identical sketch are exactly 1.0, so the strict
// float comparisons here are intentional.
#![allow(clippy::float_cmp)]

use minhash_rs::prelude::*;

const PERMUTATIONS: usize = 64;
const N: usize = 4;

#[test]
fn new_equals_default_and_starts_empty() {
    let array = MinHashArray::<u64, PERMUTATIONS, N>::new();
    assert_eq!(array, MinHashArray::<u64, PERMUTATIONS, N>::default());

    for i in 0..N {
        assert!(array[i].is_empty(), "counter {i} should start empty");
    }
}

#[test]
fn insert_is_isolated_per_index() {
    let mut array = MinHashArray::<u64, PERMUTATIONS, N>::new();

    // Insert into a single counter only.
    array[1].insert(42_u64);

    assert!(array[0].is_empty());
    assert!(!array[1].is_empty());
    assert!(array[2].is_empty());
    assert!(array[3].is_empty());

    assert!(array[1].may_contain(42_u64));
    assert!(!array[0].may_contain(42_u64));
}

#[test]
fn each_index_holds_an_independent_sketch() {
    let mut array = MinHashArray::<u64, PERMUTATIONS, N>::new();

    for i in 0..N {
        for v in 0..100_u64 {
            array[i].insert(v + (i as u64) * 1000);
        }
    }

    // Disjoint ranges should estimate a low self-vs-other Jaccard, while a
    // counter compared with itself must be exactly 1.0.
    assert_eq!(array[0].estimate_jaccard_index(&array[0]), 1.0);
    assert!(
        array[0].estimate_jaccard_index(&array[1]) < 0.2,
        "disjoint counters should have low Jaccard, got {}",
        array[0].estimate_jaccard_index(&array[1]),
    );
}

#[test]
fn index_mut_allows_overwrite() {
    let mut array = MinHashArray::<u64, PERMUTATIONS, N>::new();
    let mut replacement = MinHash::<u64, PERMUTATIONS>::new();
    replacement.insert(7_u64);

    array[2] = replacement;
    assert_eq!(array[2], replacement);
    assert!(array[2].may_contain(7_u64));
}
/// Exercise every derived trait on MinHashArray by name so the coverage tool
/// sees the monomorphised entry points.
#[test]
#[allow(
    clippy::useless_borrows_in_formatting,
    clippy::clone_on_copy,
    clippy::manual_assert_eq,
    clippy::no_effect_underscore_binding
)]
fn derived_impls_run() {
    // Default::default and new
    let a = MinHashArray::<u64, PERMUTATIONS, N>::new();
    assert_eq!(a, MinHashArray::<u64, PERMUTATIONS, N>::default());

    // Debug::fmt
    let _ = format!("{:?}", &a);

    // Clone::clone
    let cloned = a.clone();

    // PartialEq::eq
    assert!(a == cloned);

    // Index::index
    let _first: &MinHash<u64, PERMUTATIONS> = &a[0];

    // IndexMut::index_mut
    let mut b = a;
    b[0] = MinHash::<u64, PERMUTATIONS>::new();
    assert!(b[0].is_empty());
}
