//! Tests for the LSH banding API (`band_hash` and `MinHash::band_hashes`).

use minhash_rs::prelude::*;

const PERMUTATIONS: usize = 128;

#[test]
fn identical_signatures_share_all_band_hashes() {
    let left: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();
    let right: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();

    let left_bands = left.band_hashes::<16>();
    let right_bands = right.band_hashes::<16>();

    assert_eq!(left_bands, right_bands);
}

#[test]
fn band_hashes_decompose_into_band_hash() {
    let sketch: MinHash<u64, PERMUTATIONS> = (0..200u64).collect();
    let registers = sketch.as_ref();

    let bands = sketch.band_hashes::<16>();
    let rows = PERMUTATIONS / 16;

    for (band, &hash) in bands.iter().enumerate() {
        assert_eq!(
            hash,
            band_hash(&registers[band * rows..(band + 1) * rows]),
            "band {band} disagrees with band_hash over its registers",
        );
    }
}

#[test]
fn band_count_matches_const() {
    let sketch: MinHash<u64, PERMUTATIONS> = (0..50u64).collect();

    assert_eq!(sketch.band_hashes::<16>().len(), 16);
    assert_eq!(sketch.band_hashes::<8>().len(), 8);
    assert_eq!(sketch.band_hashes::<0>().len(), 0);
}

#[test]
fn non_dividing_bands_use_floor_rows() {
    let sketch: MinHash<u64, PERMUTATIONS> = (0..200u64).collect();
    let registers = sketch.as_ref();

    let bands = sketch.band_hashes::<5>();
    let rows = PERMUTATIONS / 5;
    assert_eq!(rows, 25);

    for (band, &hash) in bands.iter().enumerate() {
        assert_eq!(
            hash,
            band_hash(&registers[band * rows..(band + 1) * rows]),
            "band {band} disagrees under non-dividing bands",
        );
    }
}

#[test]
fn mutating_one_register_changes_only_its_band() {
    let mut sketch: MinHash<u64, PERMUTATIONS> = (0..200u64).collect();
    let before = sketch.band_hashes::<16>();

    let rows = PERMUTATIONS / 16;
    let index = 20;
    let changed_band = index / rows;

    let bumped = sketch[index].wrapping_add(1);
    sketch[index] = bumped;
    let after = sketch.band_hashes::<16>();

    for band in 0..16 {
        if band == changed_band {
            assert_ne!(
                before[band], after[band],
                "band {band} contains the mutated register and must change",
            );
        } else {
            assert_eq!(
                before[band], after[band],
                "band {band} does not contain the mutated register and must not change",
            );
        }
    }
}

#[test]
fn band_hash_depends_on_register_values() {
    assert_eq!(band_hash(&[1u64, 2, 3]), band_hash(&[1u64, 2, 3]));
    assert_eq!(band_hash::<u64>(&[]), band_hash::<u64>(&[]));
    assert_ne!(band_hash(&[1u64]), band_hash(&[2u64]));
    assert_ne!(band_hash(&[1u64, 2, 3]), band_hash(&[3u64, 2, 1]));
    assert_ne!(band_hash(&[1u64]), band_hash::<u64>(&[]));
}

#[test]
fn band_matches_all_indices_for_identical_signatures() {
    let left: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();
    let right: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();

    let left_bands = left.band_hashes::<16>();
    let right_bands = right.band_hashes::<16>();
    let matches: Vec<usize> = BandMatches::new(&left_bands, &right_bands).collect();
    assert_eq!(matches, (0..16).collect::<Vec<_>>());
}
#[test]
fn band_matches_partial_overlap() {
    let left: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();
    let right: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();

    let left_bands = left.band_hashes::<16>();
    let right_bands = right.band_hashes::<16>();
    // Mutate the last 4 bands of right so only bands 0..12 match.
    let mut right_bands = right_bands;
    right_bands[12] = right_bands[12].wrapping_add(1);
    right_bands[13] = right_bands[13].wrapping_add(1);
    right_bands[14] = right_bands[14].wrapping_add(1);
    right_bands[15] = right_bands[15].wrapping_add(1);

    let matches: Vec<usize> = BandMatches::new(&left_bands, &right_bands).collect();
    assert_eq!(matches, (0..12).collect::<Vec<_>>());
}

#[test]
fn band_matches_empty_when_no_overlap() {
    let left: MinHash<u64, PERMUTATIONS> = (0..100u64).collect();
    let right: MinHash<u64, PERMUTATIONS> = (100..200u64).collect();

    let left_bands = left.band_hashes::<16>();
    let right_bands = right.band_hashes::<16>();
    let matches: Vec<usize> = BandMatches::new(&left_bands, &right_bands).collect();

    assert!(matches.is_empty());
}
