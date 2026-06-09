//! Atomic insertion coverage for the FNV hash families and for every supported
//! word width (the single-threaded SipHasher13 path is covered in test_atomic).

use core::sync::atomic::Ordering;

use minhash_rs::prelude::*;

#[test]
fn atomic_fnv_has_no_false_negatives() {
    let mut mh = MinHash::<u64, 16>::new();
    {
        let atomic = mh.as_atomic();
        for v in 0..8u64 {
            atomic.fetch_insert_with_fnv(v, Ordering::Relaxed);
        }
    }
    for v in 0..8u64 {
        assert!(mh.may_contain_value_with_fnv(v));
    }
}

#[test]
fn atomic_keyed_fnv_has_no_false_negatives() {
    let key = 0xDEAD_BEEF;
    let mut mh = MinHash::<u64, 16>::new();
    {
        let atomic = mh.as_atomic();
        for v in 0..8u64 {
            atomic.fetch_insert_with_keyed_fnv(v, key, Ordering::Relaxed);
        }
    }
    for v in 0..8u64 {
        assert!(mh.may_contain_value_with_keyed_fnv(v, key));
    }
}

#[test]
fn atomic_fnv_matches_non_atomic_fnv() {
    let mut atomic_mh = MinHash::<u64, 16>::new();
    {
        let atomic = atomic_mh.as_atomic();
        atomic.fetch_insert_with_fnv(123_u64, Ordering::Relaxed);
    }

    let mut serial_mh = MinHash::<u64, 16>::new();
    serial_mh.insert_with_fnv(123_u64);

    assert_eq!(atomic_mh, serial_mh);
}

// One test per word width to exercise the `AtomicFetchMin::set_min` impl of each
// atomic type (AtomicU8/U16/U32/Usize; AtomicU64 is covered in test_atomic).
macro_rules! atomic_word_width_test {
    ($name:ident, $word:ty) => {
        #[test]
        fn $name() {
            let mut mh = MinHash::<$word, 16>::new();
            assert!(mh.is_empty());
            {
                let atomic = mh.as_atomic();
                for v in 0..8u64 {
                    atomic.fetch_insert_with_siphashes13(v, Ordering::Relaxed);
                }
            }
            assert!(!mh.is_empty());
            for v in 0..8u64 {
                assert!(mh.may_contain_value_with_siphashes13(v));
            }
        }
    };
}

atomic_word_width_test!(atomic_word_width_u8, u8);
atomic_word_width_test!(atomic_word_width_u16, u16);
atomic_word_width_test!(atomic_word_width_u32, u32);
atomic_word_width_test!(atomic_word_width_usize, usize);
