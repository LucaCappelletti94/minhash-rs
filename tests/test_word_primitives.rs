//! Exact-value tests for the per-word `XorShift` and `Primitive` impls.
//!
//! The sketch tests only assert statistical invariants, leaving the low-level
//! word operations free to be subtly wrong unnoticed. These pin each impl
//! against an independent reference. Golden values were computed from the

use minhash_rs::prelude::*;

#[test]
fn xorshift_produces_exact_sequence_values() {
    assert_eq!(1_u64.xorshift(), 1_082_269_761);
    assert_eq!(12_345_u64.xorshift(), 13_289_605_635_609);

    assert_eq!(1_u32.xorshift(), 270_369);
    assert_eq!(12_345_u32.xorshift(), 3_336_926_330);

    assert_eq!(1_u16.xorshift(), 8_225);
    assert_eq!(12_345_u16.xorshift(), 29_818);

    assert_eq!(1_u8.xorshift(), 27);
    // 17 is odd and sets bit 7 before the `>> 7` step, distinguishing `^=`
    // from `|=` there.
    assert_eq!(17_u8.xorshift(), 168);

    assert_eq!(1_usize.xorshift(), 1_082_269_761);
    assert_eq!(12_345_usize.xorshift(), 13_289_605_635_609);
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
    // u8 xorshift can produce zero from nonzero input (e.g. 128 -> 0).
    // The hash stream must remap zero to one so the sketch stays non-degenerate.
    let mut mh = MinHash::<u8, 16>::new();
    // Insert enough values to trigger zero in the xorshift stream.
    // 128_u8.xorshift() == 0, so any seed that derives to 128 will hit zero.
    for i in 0..=255u8 {
        mh.insert_with_fnv(u64::from(i));
    }
    // No word should be zero (zero is remapped to one).
    for &word in mh.as_ref() {
        assert_ne!(word, 0, "zero leaked into MinHash words");
    }
    // The sketch should not be empty (inserts produced valid hashes).
    assert!(!mh.is_empty());
}

#[test]
fn hash_stream_produces_diverse_values() {
    // If the zero-check guard were inverted (== -> !=), all non-zero hashes
    // would be remapped to one, making every sketch identical and Jaccard
    // estimates collapse to 1.0 regardless of actual similarity.
    let mut a = MinHash::<u64, 128>::new();
    let mut b = MinHash::<u64, 128>::new();

    for i in 0..1000u64 {
        a.insert_with_fnv(i);
    }
    for i in 1000..2000u64 {
        b.insert_with_fnv(i);
    }

    // Two disjoint sets should have Jaccard near 0, not 1.
    let jaccard = a.estimate_jaccard_index(&b);
    assert!(jaccard < 0.1, "Jaccard {jaccard} is too high for disjoint sets");
}
