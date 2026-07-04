//! Tests for `SparseArithmetic` and `HashType` trait implementations.
//!
//! Every operation is called through fully-qualified syntax so that the
//! coverage tool tracks the monomorphisation by name.

use minhash_rs::hashtype::{HashType, SparseArithmetic};

// ---------------------------------------------------------------------------
// SparseArithmetic: wrapping_add
// ---------------------------------------------------------------------------

#[test]
fn wrapping_add_u64_normal() {
    assert_eq!(<u64 as SparseArithmetic>::wrapping_add(5, 3), 8);
    assert_eq!(<u64 as SparseArithmetic>::wrapping_add(0, 0), 0);
    assert_eq!(<u64 as SparseArithmetic>::wrapping_add(100, 200), 300);
}

#[test]
fn wrapping_add_u64_wrap_at_max() {
    assert_eq!(<u64 as SparseArithmetic>::wrapping_add(u64::MAX, 1), 0);
    assert_eq!(<u64 as SparseArithmetic>::wrapping_add(u64::MAX, 2), 1);
    assert_eq!(
        <u64 as SparseArithmetic>::wrapping_add(u64::MAX, u64::MAX),
        u64::MAX.wrapping_sub(1)
    );
}

#[test]
fn wrapping_add_u32_normal() {
    assert_eq!(<u32 as SparseArithmetic>::wrapping_add(5, 3), 8);
    assert_eq!(<u32 as SparseArithmetic>::wrapping_add(0, 0), 0);
}

#[test]
fn wrapping_add_u32_wrap_at_max() {
    assert_eq!(<u32 as SparseArithmetic>::wrapping_add(u32::MAX, 1), 0);
    assert_eq!(<u32 as SparseArithmetic>::wrapping_add(u32::MAX, 2), 1);
}

// ---------------------------------------------------------------------------
// SparseArithmetic: wrapping_sub
// ---------------------------------------------------------------------------

#[test]
fn wrapping_sub_u64_normal() {
    assert_eq!(<u64 as SparseArithmetic>::wrapping_sub(10, 3), 7);
    assert_eq!(<u64 as SparseArithmetic>::wrapping_sub(0, 0), 0);
}

#[test]
fn wrapping_sub_u64_wrap_at_zero() {
    assert_eq!(<u64 as SparseArithmetic>::wrapping_sub(0, 1), u64::MAX);
    assert_eq!(<u64 as SparseArithmetic>::wrapping_sub(0, 2), u64::MAX - 1);
}

#[test]
fn wrapping_sub_u32_normal() {
    assert_eq!(<u32 as SparseArithmetic>::wrapping_sub(10, 3), 7);
    assert_eq!(<u32 as SparseArithmetic>::wrapping_sub(0, 0), 0);
}

#[test]
fn wrapping_sub_u32_wrap_at_zero() {
    assert_eq!(<u32 as SparseArithmetic>::wrapping_sub(0, 1), u32::MAX);
    assert_eq!(<u32 as SparseArithmetic>::wrapping_sub(0, 2), u32::MAX - 1);
}

// ---------------------------------------------------------------------------
// SparseArithmetic: saturating_add
// ---------------------------------------------------------------------------

#[test]
fn saturating_add_u64_normal() {
    assert_eq!(<u64 as SparseArithmetic>::saturating_add(5, 3), 8);
    assert_eq!(<u64 as SparseArithmetic>::saturating_add(0, 0), 0);
}

#[test]
fn saturating_add_u64_clamp_at_max() {
    assert_eq!(
        <u64 as SparseArithmetic>::saturating_add(u64::MAX, 1),
        u64::MAX
    );
    assert_eq!(
        <u64 as SparseArithmetic>::saturating_add(u64::MAX, 100),
        u64::MAX
    );
}

#[test]
fn saturating_add_u32_normal() {
    assert_eq!(<u32 as SparseArithmetic>::saturating_add(5, 3), 8);
}

#[test]
fn saturating_add_u32_clamp_at_max() {
    assert_eq!(
        <u32 as SparseArithmetic>::saturating_add(u32::MAX, 1),
        u32::MAX
    );
}

// ---------------------------------------------------------------------------
// SparseArithmetic: saturating_sub
// ---------------------------------------------------------------------------

#[test]
fn saturating_sub_u64_normal() {
    assert_eq!(<u64 as SparseArithmetic>::saturating_sub(10, 3), 7);
    assert_eq!(<u64 as SparseArithmetic>::saturating_sub(0, 0), 0);
}

#[test]
fn saturating_sub_u64_clamp_at_zero() {
    assert_eq!(<u64 as SparseArithmetic>::saturating_sub(0, 1), 0);
    assert_eq!(<u64 as SparseArithmetic>::saturating_sub(5, 10), 0);
}

#[test]
fn saturating_sub_u32_normal() {
    assert_eq!(<u32 as SparseArithmetic>::saturating_sub(10, 3), 7);
}

#[test]
fn saturating_sub_u32_clamp_at_zero() {
    assert_eq!(<u32 as SparseArithmetic>::saturating_sub(0, 1), 0);
}

// ---------------------------------------------------------------------------
// HashType: from_u64_digest
// ---------------------------------------------------------------------------

#[test]
fn from_u64_digest_u64_identity() {
    assert_eq!(<u64 as HashType>::from_u64_digest(0), 0);
    assert_eq!(<u64 as HashType>::from_u64_digest(u64::MAX), u64::MAX);
    assert_eq!(
        <u64 as HashType>::from_u64_digest(0xdead_beef_cafe_babe),
        0xdead_beef_cafe_babe
    );
}

#[test]
fn from_u64_digest_u32_truncates() {
    // u32 truncates to the low 32 bits
    let digest: u64 = (0x1234_5678 << 32) | 0xdead_beef;
    assert_eq!(<u32 as HashType>::from_u64_digest(digest), 0xdead_beef);
    assert_eq!(<u32 as HashType>::from_u64_digest(u64::MAX), u32::MAX);
    assert_eq!(<u32 as HashType>::from_u64_digest(0), 0);
}

#[test]
fn from_u64_digest_u32_deterministic() {
    let digest = 0xabcd_ef01_2345_6789;
    let a = <u32 as HashType>::from_u64_digest(digest);
    let b = <u32 as HashType>::from_u64_digest(digest);
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// HashType: splitmix
// ---------------------------------------------------------------------------

#[test]
fn splitmix_u64_not_identity() {
    let seed: u64 = 42;
    let mixed = <u64 as HashType>::splitmix(seed);
    assert_ne!(mixed, seed, "splitmix must not be the identity");
    assert_ne!(mixed, 0, "splitmix of nonzero seed must not produce zero");
}

#[test]
fn splitmix_u64_varied_inputs() {
    let a = <u64 as HashType>::splitmix(1);
    let b = <u64 as HashType>::splitmix(2);
    assert_ne!(
        a, b,
        "splitmix must produce different outputs for different inputs"
    );
}

#[test]
fn splitmix_u32_not_identity() {
    let seed: u32 = 42;
    let mixed = <u32 as HashType>::splitmix(seed);
    assert_ne!(mixed, seed, "splitmix must not be the identity");
    assert_ne!(mixed, 0, "splitmix of nonzero seed must not produce zero");
}

#[test]
fn splitmix_u32_varied_inputs() {
    let a = <u32 as HashType>::splitmix(1);
    let b = <u32 as HashType>::splitmix(2);
    assert_ne!(
        a, b,
        "splitmix must produce different outputs for different inputs"
    );
}

#[test]
fn splitmix_u32_and_u64_differ() {
    // The u32 mixer (lowbias32) uses different constants than the u64
    // mixer (SplitMix64), so the same numeric seed must produce different
    // results between the two widths.
    let seed: u64 = 12345;
    let u64_mixed = <u64 as HashType>::splitmix(seed);
    let u32_mixed = u64::from(<u32 as HashType>::splitmix(seed as u32));
    assert_ne!(
        u64_mixed, u32_mixed,
        "u32 and u64 splitmix must differ for the same seed"
    );
}
