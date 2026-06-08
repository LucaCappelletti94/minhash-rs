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
    let mut minhash = MinHash::<u64, 8>::new();
    assert!(minhash.is_empty());

    {
        let atomic = minhash.as_atomic();
        atomic.fetch_insert_with_siphashes13(42, Ordering::Relaxed);
        atomic.fetch_insert_with_siphashes13(47, Ordering::Relaxed);
    }

    assert!(!minhash.is_empty());
    assert!(minhash.may_contain_value_with_siphashes13(42));
    assert!(minhash.may_contain_value_with_siphashes13(47));
}

#[test]
fn atomic_insert_keyed_single_threaded_matches_membership() {
    let key0 = 0x0123_4567_89AB_CDEF;
    let key1 = 0xFEDC_BA98_7654_3210;

    let mut minhash = MinHash::<u64, 8>::new();
    {
        let atomic = minhash.as_atomic();
        atomic.fetch_insert_with_keyed_siphashes13(42, key0, key1, Ordering::Relaxed);
    }
    assert!(minhash.may_contain_value_with_keyed_siphashes13(42, key0, key1));
}

#[test]
fn atomic_insert_matches_non_atomic_insert() {
    // The atomic and non-atomic paths share the hash generator, so a single
    // value inserted either way must yield identical words.
    let mut atomic_mh = MinHash::<u64, 16>::new();
    {
        let atomic = atomic_mh.as_atomic();
        atomic.fetch_insert_with_siphashes13(123_u64, Ordering::Relaxed);
    }

    let mut serial_mh = MinHash::<u64, 16>::new();
    serial_mh.insert_with_siphashes13(123_u64);

    assert_eq!(atomic_mh, serial_mh);
}

#[test]
fn atomic_insert_concurrent_inserts_all_values() {
    // Keep the sizes tiny so this stays fast under the Miri interpreter.
    let mut minhash = MinHash::<u64, 16>::new();
    let values: Vec<u64> = (0..8).collect();

    {
        let atomic = minhash.as_atomic();
        thread::scope(|scope| {
            for &value in &values {
                scope.spawn(move || {
                    atomic.fetch_insert_with_siphashes13(value, Ordering::Relaxed);
                });
            }
        });
    }

    // Every concurrently inserted value must be reported as possibly present.
    for &value in &values {
        assert!(
            minhash.may_contain_value_with_siphashes13(value),
            "value {value} was inserted concurrently but is not contained",
        );
    }
}
