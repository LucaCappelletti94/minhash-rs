//! Exact-value tests for the per-word `XorShift`, `Primitive` and `Min` impls.
//!
//! The sketch tests only assert statistical invariants, leaving the low-level
//! word operations free to be subtly wrong unnoticed. These pin each impl
//! against an independent reference. Golden values were computed from the
//! xorshift recurrence; usize routes through u64 and u16 through u32.

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
fn is_min_reflects_the_ordering() {
    assert!(Min::is_min(&3_usize, 5));
    assert!(Min::is_min(&5_usize, 5));
    assert!(!Min::is_min(&5_usize, 3));

    assert!(Min::is_min(&3_u64, 5));
    assert!(!Min::is_min(&5_u64, 3));

    assert!(Min::is_min(&3_u32, 5));
    assert!(!Min::is_min(&5_u32, 3));

    assert!(Min::is_min(&3_u16, 5));
    assert!(!Min::is_min(&5_u16, 3));

    assert!(Min::is_min(&3_u8, 5));
    assert!(!Min::is_min(&5_u8, 3));
}

#[test]
fn set_min_keeps_the_smaller_value_usize() {
    let mut w = 5_usize;
    w.set_min(2);
    assert_eq!(w, 2);
}

#[test]
fn set_min_keeps_the_smaller_value_u64() {
    let mut w = 5_u64;
    w.set_min(2);
    assert_eq!(w, 2);
}

#[test]
fn set_min_keeps_the_smaller_value_u16() {
    let mut w = 5_u16;
    w.set_min(2);
    assert_eq!(w, 2);
}

#[test]
fn set_min_keeps_the_smaller_value_u8() {
    let mut w = 5_u8;
    w.set_min(2);
    assert_eq!(w, 2);
}

#[test]
fn set_min_keeps_the_smaller_value_u32() {
    let mut w = 5_u32;
    w.set_min(9);
    assert_eq!(w, 5);
    w.set_min(2);
    assert_eq!(w, 2);
}
