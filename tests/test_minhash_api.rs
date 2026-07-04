//! Tests for the non-atomic MinHash API surface: construction, permutation
//! count, indexing, and slice views.

use minhash_rs::prelude::*;

const PERMUTATIONS: usize = 128;

#[test]
fn default_equals_new() {
    assert_eq!(
        MinHash::<u64, PERMUTATIONS>::default(),
        MinHash::<u64, PERMUTATIONS>::new()
    );
}

#[test]
fn number_of_permutations_reports_const() {
    assert_eq!(
        MinHash::<u64, PERMUTATIONS>::new().number_of_permutations(),
        PERMUTATIONS
    );
    assert_eq!(MinHash::<u32, 64>::new().number_of_permutations(), 64);
}

#[test]
fn indexing_and_slice_views_expose_words() {
    let mut mh = MinHash::<u64, 8>::new();

    // A fresh sketch is all maximal sentinels, visible via Index and AsRef.
    assert_eq!(mh[0], u64::MAX);
    assert_eq!(mh.as_ref().len(), 8);
    assert!(mh.as_ref().iter().all(|&w| w == u64::MAX));

    // IndexMut and AsMut both allow direct word mutation.
    mh[0] = 123;
    assert_eq!(mh[0], 123);
    assert_eq!(mh.as_ref()[0], 123);

    mh.as_mut()[1] = 456;
    assert_eq!(mh[1], 456);
}
