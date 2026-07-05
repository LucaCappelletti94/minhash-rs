//! Exhaustive coverage for every `Primitive::convert` monomorphisation.
//!
//! Each test exercises one source type across all target types the impl
//! provides, using fully-qualified calls so llvm-cov attributes every
//! monomorphisation by name.

use minhash_rs::prelude::Primitive;

#[test]
fn convert_from_u64() {
    // Values chosen so that every narrowing target sees distinct low bits,
    // not just zero or one.
    let wide = 0x0123_4567_89ab_cdef_u64;

    assert_eq!(<u64 as Primitive<u8>>::convert(wide), 0xef_u8);
    assert_eq!(<u64 as Primitive<u16>>::convert(wide), 0xCDEF_u16);
    assert_eq!(<u64 as Primitive<u32>>::convert(wide), 0x89AB_CDEF_u32);
    assert_eq!(<u64 as Primitive<u64>>::convert(wide), wide);
    assert_eq!(<u64 as Primitive<usize>>::convert(wide), wide as usize);

    // Boundary: zero
    assert_eq!(<u64 as Primitive<u8>>::convert(0), 0);
    assert_eq!(<u64 as Primitive<u16>>::convert(0), 0);
    assert_eq!(<u64 as Primitive<u32>>::convert(0), 0);
    assert_eq!(<u64 as Primitive<u64>>::convert(0), 0);
    assert_eq!(<u64 as Primitive<usize>>::convert(0), 0);

    // Boundary: one
    assert_eq!(<u64 as Primitive<u8>>::convert(1), 1);
    assert_eq!(<u64 as Primitive<u16>>::convert(1), 1);
    assert_eq!(<u64 as Primitive<u32>>::convert(1), 1);
    assert_eq!(<u64 as Primitive<u64>>::convert(1), 1);
    assert_eq!(<u64 as Primitive<usize>>::convert(1), 1);

    // Boundary: max (truncates to all-ones in each narrower width)
    assert_eq!(<u64 as Primitive<u8>>::convert(u64::MAX), u8::MAX);
    assert_eq!(<u64 as Primitive<u16>>::convert(u64::MAX), u16::MAX);
    assert_eq!(<u64 as Primitive<u32>>::convert(u64::MAX), u32::MAX);
    assert_eq!(<u64 as Primitive<u64>>::convert(u64::MAX), u64::MAX);
}

#[test]
fn convert_from_u32() {
    let mid = 0xDEAD_BEEF_u32;

    assert_eq!(<u32 as Primitive<u8>>::convert(mid), 0xEF_u8);
    assert_eq!(<u32 as Primitive<u16>>::convert(mid), 0xBEEF_u16);
    assert_eq!(<u32 as Primitive<u32>>::convert(mid), mid);
    assert_eq!(<u32 as Primitive<u64>>::convert(mid), 0xDEAD_BEEF_u64);
    assert_eq!(<u32 as Primitive<usize>>::convert(mid), mid as usize);

    // Boundary: zero
    assert_eq!(<u32 as Primitive<u8>>::convert(0), 0);
    assert_eq!(<u32 as Primitive<u16>>::convert(0), 0);
    assert_eq!(<u32 as Primitive<u32>>::convert(0), 0);
    assert_eq!(<u32 as Primitive<u64>>::convert(0), 0);
    assert_eq!(<u32 as Primitive<usize>>::convert(0), 0);

    // Boundary: one
    assert_eq!(<u32 as Primitive<u8>>::convert(1), 1);
    assert_eq!(<u32 as Primitive<u16>>::convert(1), 1);
    assert_eq!(<u32 as Primitive<u32>>::convert(1), 1);
    assert_eq!(<u32 as Primitive<u64>>::convert(1), 1);
    assert_eq!(<u32 as Primitive<usize>>::convert(1), 1);

    // Boundary: max
    assert_eq!(<u32 as Primitive<u8>>::convert(u32::MAX), u8::MAX);
    assert_eq!(<u32 as Primitive<u16>>::convert(u32::MAX), u16::MAX);
    assert_eq!(<u32 as Primitive<u32>>::convert(u32::MAX), u32::MAX);
    assert_eq!(
        <u32 as Primitive<u64>>::convert(u32::MAX),
        u64::from(u32::MAX)
    );
}

#[test]
fn convert_from_u16() {
    let narrow = 0x1234_u16;

    assert_eq!(<u16 as Primitive<u32>>::convert(narrow), 0x1234_u32);
    assert_eq!(<u16 as Primitive<u64>>::convert(narrow), 0x1234_u64);

    // Boundary: zero
    assert_eq!(<u16 as Primitive<u32>>::convert(0), 0);
    assert_eq!(<u16 as Primitive<u64>>::convert(0), 0);

    // Boundary: one
    assert_eq!(<u16 as Primitive<u32>>::convert(1), 1);
    assert_eq!(<u16 as Primitive<u64>>::convert(1), 1);

    // Boundary: max
    assert_eq!(
        <u16 as Primitive<u32>>::convert(u16::MAX),
        u32::from(u16::MAX)
    );
    assert_eq!(
        <u16 as Primitive<u64>>::convert(u16::MAX),
        u64::from(u16::MAX)
    );
}

#[test]
fn convert_from_u8() {
    let tiny = 0xAB_u8;

    assert_eq!(<u8 as Primitive<u32>>::convert(tiny), 0xAB_u32);
    assert_eq!(<u8 as Primitive<u64>>::convert(tiny), 0xAB_u64);

    // Boundary: zero
    assert_eq!(<u8 as Primitive<u32>>::convert(0), 0);
    assert_eq!(<u8 as Primitive<u64>>::convert(0), 0);

    // Boundary: one
    assert_eq!(<u8 as Primitive<u32>>::convert(1), 1);
    assert_eq!(<u8 as Primitive<u64>>::convert(1), 1);

    // Boundary: max
    assert_eq!(<u8 as Primitive<u32>>::convert(u8::MAX), u32::from(u8::MAX));
    assert_eq!(<u8 as Primitive<u64>>::convert(u8::MAX), u64::from(u8::MAX));
}

#[test]
fn convert_from_usize() {
    let ptr = 0x0123_4567_89ab_cdef_usize;

    assert_eq!(<usize as Primitive<u32>>::convert(ptr), ptr as u32);
    assert_eq!(<usize as Primitive<u64>>::convert(ptr), ptr as u64);

    // Boundary: zero
    assert_eq!(<usize as Primitive<u32>>::convert(0), 0);
    assert_eq!(<usize as Primitive<u64>>::convert(0), 0);

    // Boundary: one
    assert_eq!(<usize as Primitive<u32>>::convert(1), 1);
    assert_eq!(<usize as Primitive<u64>>::convert(1), 1);

    // Boundary: max (truncates to u32::MAX on 64-bit)
    assert_eq!(<usize as Primitive<u32>>::convert(usize::MAX), u32::MAX);
}

/// Pin the `IS_LOSSLESS` const for every impl in the crate. Any change
/// to the default computation on the trait, or to any impl's override,
/// flips at least one of these asserts and the crate stops compiling.
#[test]
fn is_lossless_matches_width_relation() {
    // Identity: always lossless.
    const _: () = assert!(<u64 as Primitive<u64>>::IS_LOSSLESS);
    const _: () = assert!(<u32 as Primitive<u32>>::IS_LOSSLESS);

    // Widening: lossless.
    const _: () = assert!(<u32 as Primitive<u64>>::IS_LOSSLESS);
    const _: () = assert!(<u32 as Primitive<usize>>::IS_LOSSLESS);
    const _: () = assert!(<u8 as Primitive<u32>>::IS_LOSSLESS);
    const _: () = assert!(<u8 as Primitive<u64>>::IS_LOSSLESS);
    const _: () = assert!(<u16 as Primitive<u32>>::IS_LOSSLESS);
    const _: () = assert!(<u16 as Primitive<u64>>::IS_LOSSLESS);

    // Narrowing: not lossless.
    const _: () = assert!(!<u64 as Primitive<u8>>::IS_LOSSLESS);
    const _: () = assert!(!<u64 as Primitive<u16>>::IS_LOSSLESS);
    const _: () = assert!(!<u64 as Primitive<u32>>::IS_LOSSLESS);
    const _: () = assert!(!<u32 as Primitive<u8>>::IS_LOSSLESS);
    const _: () = assert!(!<u32 as Primitive<u16>>::IS_LOSSLESS);

    // `usize`: width depends on target. Assert relative to `u64`.
    #[cfg(target_pointer_width = "64")]
    const _: () = {
        assert!(<usize as Primitive<u64>>::IS_LOSSLESS);
        assert!(!<usize as Primitive<u32>>::IS_LOSSLESS);
        assert!(<u64 as Primitive<usize>>::IS_LOSSLESS);
    };
    #[cfg(not(target_pointer_width = "64"))]
    const _: () = {
        assert!(<usize as Primitive<u64>>::IS_LOSSLESS);
        assert!(<usize as Primitive<u32>>::IS_LOSSLESS);
        assert!(!<u64 as Primitive<usize>>::IS_LOSSLESS);
    };
}
