//! Tests for the non-atomic MinHash API surface: the keyed SipHasher13 and FNV
//! insert/membership variants, plus the small accessors (construction,
//! permutation count, indexing and slice views).

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
fn keyed_siphash13_has_no_false_negatives() {
    let key0 = 0x0123_4567_89AB_CDEF;
    let key1 = 0xFEDC_BA98_7654_3210;

    let mut mh = MinHash::<u64, PERMUTATIONS, SipHashes13Keyed>::new_with_keys(key0, key1);
    let values: Vec<u64> = (0..50).collect();
    for &v in &values {
        mh.insert(v);
    }
    for &v in &values {
        assert!(mh.may_contain(v));
    }
}

#[test]
fn keyed_fnv_has_no_false_negatives() {
    let key = 0x0123_4567_89AB_CDEF;

    let mut mh = MinHash::<u64, PERMUTATIONS, FnvKeyed>::new_with_keys(key, 0);
    let values: Vec<u64> = (0..50).collect();
    for &v in &values {
        mh.insert(v);
    }
    for &v in &values {
        assert!(mh.may_contain(v));
    }
}

#[test]
fn keyed_siphash13_same_key_matches_different_key_differs() {
    let values: Vec<u64> = (0..200).collect();

    let mut a = MinHash::<u64, PERMUTATIONS, SipHashes13Keyed>::new_with_keys(1, 2);
    let mut same = MinHash::<u64, PERMUTATIONS, SipHashes13Keyed>::new_with_keys(1, 2);
    let mut other = MinHash::<u64, PERMUTATIONS, SipHashes13Keyed>::new_with_keys(3, 4);
    for &v in &values {
        a.insert(v);
        same.insert(v);
        other.insert(v);
    }

    assert_eq!(a, same, "the same key over the same values must match");
    assert_ne!(a, other, "different keys should produce different sketches");
}

#[test]
fn keyed_fnv_different_keys_differ() {
    let values: Vec<u64> = (0..200).collect();

    let mut a = MinHash::<u64, PERMUTATIONS, FnvKeyed>::new_with_keys(7, 0);
    let mut b = MinHash::<u64, PERMUTATIONS, FnvKeyed>::new_with_keys(99, 0);
    for &v in &values {
        a.insert(v);
        b.insert(v);
    }
    assert_ne!(a, b);
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
