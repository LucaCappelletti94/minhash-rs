//! Tests that the MinHash invariants hold across every supported word type.
//!
//! The defining guarantee of MinHash is that membership queries never produce
//! false negatives: any value that was inserted must be reported as possibly
//! contained. This must hold for every word width, exercising the per-width
//! `XorShift`, `Primitive` and `Maximal` implementations.

use minhash_rs::prelude::*;

fn no_false_negatives<Word, const PERMUTATIONS: usize>(values: &[u64])
where
    Word: Min + XorShift + Copy + Eq + Maximal,
    u64: Primitive<Word>,
{
    let mut mh = MinHash::<Word, PERMUTATIONS>::new();
    assert!(mh.is_empty());

    for &v in values {
        mh.insert_with_siphashes13(v);
        mh.insert_with_fnv(v);
    }
    assert!(!mh.is_empty());

    for &v in values {
        assert!(
            mh.may_contain_value_with_siphashes13(v),
            "siphash false negative for {v} with {} bits",
            mh.memory() / PERMUTATIONS,
        );
        assert!(
            mh.may_contain_value_with_fnv(v),
            "fnv false negative for {v} with {} bits",
            mh.memory() / PERMUTATIONS,
        );
    }

    // A sketch is always perfectly similar to itself.
    assert_eq!(mh.estimate_jaccard_index(&mh), 1.0);
}

#[test]
fn no_false_negatives_across_word_types() {
    let values: Vec<u64> = (0..32).collect();
    no_false_negatives::<u8, 64>(&values);
    no_false_negatives::<u16, 64>(&values);
    no_false_negatives::<u32, 64>(&values);
    no_false_negatives::<u64, 64>(&values);
    no_false_negatives::<usize, 64>(&values);
}

#[test]
fn memory_matches_word_width() {
    assert_eq!(MinHash::<u8, 128>::new().memory(), 128 * 8);
    assert_eq!(MinHash::<u16, 128>::new().memory(), 128 * 16);
    assert_eq!(MinHash::<u32, 128>::new().memory(), 128 * 32);
    assert_eq!(MinHash::<u64, 128>::new().memory(), 128 * 64);
}

#[test]
fn small_words_saturate_and_report_full() {
    // A u8 sketch has few distinct hash values per permutation, so a large
    // universe drives every word down to the minimum hash value (one, since
    // zero is never emitted), at which point the sketch is full.
    let mut mh = MinHash::<u8, 16>::new();
    assert!(!mh.is_full());
    for i in 0..100_000_u64 {
        mh.insert_with_siphashes13(i);
    }
    assert!(mh.is_full());
}

#[test]
fn single_value_never_saturates_small_words() {
    // Regression test: previously about one value in 256 truncated to a zero
    // seed and, because XorShift fixes zero, collapsed an entire u8 sketch to
    // full from a single insertion. No single value may saturate the sketch.
    for v in 0..5_000_u64 {
        let mut mh = MinHash::<u8, 32>::new();
        mh.insert_with_siphashes13(v);
        assert!(!mh.is_full(), "value {v} saturated the sketch on its own");
    }
}
