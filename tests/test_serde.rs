//! Serde round-trip tests for MinHash and MinHashArray.

use minhash_rs::prelude::*;

fn roundtrip_minhash<Word, const PERMUTATIONS: usize>(values: &[u64])
where
    Word: Min
        + XorShift
        + Copy
        + Ord
        + Eq
        + Maximal
        + core::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned,
    u64: Primitive<Word>,
{
    let mut original = MinHash::<Word, PERMUTATIONS>::new();
    for &v in values {
        original.insert_with_siphashes13(v);
    }

    let json = serde_json::to_string(&original).expect("serialization failed");
    let decoded: MinHash<Word, PERMUTATIONS> =
        serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(original, decoded);
}

#[test]
fn minhash_roundtrips_across_word_types() {
    let values: Vec<u64> = (0..50).collect();
    roundtrip_minhash::<u8, 128>(&values);
    roundtrip_minhash::<u16, 128>(&values);
    roundtrip_minhash::<u32, 128>(&values);
    roundtrip_minhash::<u64, 128>(&values);
    roundtrip_minhash::<usize, 128>(&values);
}

#[test]
fn minhash_roundtrips_large_permutation_count() {
    // Exercises the BigArray path well beyond serde's built-in array limit (32).
    let values: Vec<u64> = (0..1000).collect();
    roundtrip_minhash::<u64, 4096>(&values);
}

#[test]
fn minhash_array_roundtrips() {
    const PERMUTATIONS: usize = 64;
    const N: usize = 8;

    let mut original = MinHashArray::<u64, PERMUTATIONS, N>::new();
    for i in 0..N {
        for v in 0..20_u64 {
            original[i].insert_with_siphashes13(v + (i as u64) * 100);
        }
    }

    let json = serde_json::to_string(&original).expect("serialization failed");
    let decoded: MinHashArray<u64, PERMUTATIONS, N> =
        serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(original, decoded);
}
