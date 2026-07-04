//! Tests exercising the atomic insertion path of MinHash.
//!
//! These tests drive the safe `from_mut_slice` based atomic view so that
//! running them under Miri (`cargo +nightly miri test`) checks the soundness
//! of the atomic insertion machinery, including concurrent inserts (Miri's
//! data-race detector is enabled by default).

use core::sync::atomic::Ordering;
use std::thread;

use minhash_rs::prelude::*;

#[test]
fn atomic_insert_single_threaded_matches_membership() {
    let mut minhash = MinHash::<u64, 8, SipHashes13>::new();
    assert!(minhash.is_empty());

    {
        let atomic = minhash.as_atomic();
        atomic.fetch_insert_with_siphashes13::<u64, u64>(42u64, Ordering::Relaxed);
        atomic.fetch_insert_with_siphashes13::<u64, u64>(47u64, Ordering::Relaxed);
    }

    assert!(!minhash.is_empty());
    assert!(minhash.may_contain(42u64));
    assert!(minhash.may_contain(47u64));
}

#[test]
fn atomic_insert_matches_non_atomic_insert() {
    // The atomic and non-atomic paths share the hash generator, so a single
    // value inserted either way must yield identical words.
    let mut atomic_mh = MinHash::<u64, 16, SipHashes13>::new();
    {
        let atomic = atomic_mh.as_atomic();
        atomic.fetch_insert_with_siphashes13::<u64, u64>(123_u64, Ordering::Relaxed);
    }

    let mut serial_mh = MinHash::<u64, 16, SipHashes13>::new();
    serial_mh.insert(123_u64);

    assert_eq!(atomic_mh, serial_mh);
}

#[test]
fn atomic_insert_concurrent_inserts_all_values() {
    // Keep the sizes tiny so this stays fast under the Miri interpreter.
    let mut minhash = MinHash::<u64, 16, SipHashes13>::new();
    let values: Vec<u64> = (0..8).collect();

    {
        let atomic = minhash.as_atomic();
        thread::scope(|scope| {
            for &value in &values {
                scope.spawn(move || {
                    atomic.fetch_insert_with_siphashes13::<u64, u64>(value, Ordering::Relaxed);
                });
            }
        });
    }

    // Every concurrently inserted value must be reported as possibly present.
    for &value in &values {
        assert!(
            minhash.may_contain(value),
            "value {value} was inserted concurrently but is not contained",
        );
    }
}
#[test]
fn iter_hashes_from_value_matches_generic_and_specialised() {
    let sip_iter: Vec<u64> = MinHash::<u64, 32>::iter_siphashes13_from_value(42u64).collect();
    let generic_iter: Vec<u64> =
        MinHash::<u64, 32>::iter_hashes_from_value(42u64, siphasher::sip128::SipHasher13::new())
            .collect();
    let fnv_iter: Vec<u64> = MinHash::<u64, 32>::iter_fnv_from_value(42u64).collect();

    assert_eq!(
        sip_iter, generic_iter,
        "siphashes13 and generic with SipHasher13 must produce identical hashes",
    );
    assert_ne!(
        sip_iter, fnv_iter,
        "SipHash and FNV must produce different hash sequences",
    );
}

#[test]
fn fetch_insert_with_siphashes13_membership() {
    let mut mh = MinHash::<u64, 32, SipHashes13>::new();
    {
        let atomic = mh.as_atomic();
        atomic.fetch_insert_with_siphashes13::<u64, u64>(42u64, Ordering::Relaxed);
    }
    assert!(mh.may_contain(42u64));
}

#[test]
fn fetch_insert_with_fnv_membership() {
    let mut mh = MinHash::<u64, 32, Fnv>::new();
    {
        let atomic = mh.as_atomic();
        atomic.fetch_insert_with_fnv::<u64, u64>(42u64, Ordering::Relaxed);
    }
    assert!(mh.may_contain(42u64));
}
