//! Exact-value tests for the per-hash `XorShift` and the per-word `Primitive`
//! impls.
//!
//! MinHash iterates the permutation stream on the `HashType` (`u64` or `u32`),
//! not on the `Word`, so the pinned XorShift golden values live here for the
//! two hash widths only. Word-side conversion has its own pinned values
//! below.

use minhash_rs::prelude::*;

#[test]
fn xorshift_produces_exact_sequence_values() {
    assert_eq!(1_u64.xorshift(), 1_082_269_761);
    assert_eq!(12_345_u64.xorshift(), 13_289_605_635_609);

    assert_eq!(1_u32.xorshift(), 270_369);
    assert_eq!(12_345_u32.xorshift(), 3_336_926_330);
}

#[test]
fn xorshift_is_not_the_identity_or_constant() {
    for &v in &[1_u64, 7, 0xABCD, 0xDEAD_BEEF] {
        let out = v.xorshift();
        assert_ne!(out, 0, "u64 xorshift collapsed {v} to zero");
        assert_ne!(out, v, "u64 xorshift was the identity for {v}");
    }
}

#[test]
fn primitive_convert_narrows_to_the_low_bits() {
    // Low bits are neither 0 nor 1 in any window, so a constant replacement is
    // detectable at every width.
    let value: u64 = 0xDEAD_BEEF_1234_ABCD;

    assert_eq!(Primitive::<u8>::convert(value), 0xCD);
    assert_eq!(Primitive::<u16>::convert(value), 0xABCD);
    assert_eq!(Primitive::<u32>::convert(value), 0x1234_ABCD);
    assert_eq!(Primitive::<u64>::convert(value), value);
    assert_eq!(Primitive::<usize>::convert(value), value as usize);
}

#[test]
fn zero_is_never_emitted_by_the_hash_stream() {
    // The dense hash stream lives on the `Hash` type (u64 by default) and is
    // guarded against zero both there and after the truncation to `Word`.
    // Without the post-truncation guard, an insertion whose truncated word
    // happens to be zero would park `words[0] == 0`, which is the sparse
    // mode flag and would then mislead `is_sparse`.
    let mut mh = MinHash::<u8, 16, Fnv>::new();
    for i in 0..=255u8 {
        mh.insert(u64::from(i));
    }
    for &word in mh.as_ref() {
        assert_ne!(word, 0, "zero leaked into MinHash words");
    }
    assert!(!mh.is_empty());
}

#[test]
fn hash_stream_produces_diverse_values() {
    // If the zero-check guard were inverted (== -> !=), all non-zero hashes
    // would be remapped to one, making every sketch identical and Jaccard
    // estimates collapse to 1.0 regardless of actual similarity.
    let mut a = MinHash::<u64, 128, Fnv>::new();
    let mut b = MinHash::<u64, 128, Fnv>::new();

    for i in 0..1000u64 {
        a.insert(i);
    }
    for i in 1000..2000u64 {
        b.insert(i);
    }

    // Two disjoint sets should have Jaccard near 0, not 1.
    let jaccard = a.estimate_jaccard_index(&b);
    assert!(
        jaccard < 0.1,
        "Jaccard {jaccard} is too high for disjoint sets"
    );
}
