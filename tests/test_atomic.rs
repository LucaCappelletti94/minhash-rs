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
