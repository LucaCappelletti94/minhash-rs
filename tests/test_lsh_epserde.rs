//! Round-trip pin between the in-memory [`LshIndex`] and its
//! epserde-serialised [`LshIndexRepr`] form: save to disk, load full
//! and mmap, then verify candidate query results match the in-memory
//! index on the same queries.
//!
//! Also pins the true zero-copy claim: after `mmap`, the deserialised
//! `LshIndexRepr::DeserType<'_>` has `band_entries: &[BandEntry]`, not
//! `Vec<BandEntry>`. That equality holds at the type level, so if the
//! derive ever regresses to full-copying the entries field, this file
//! fails to compile.

#![cfg(feature = "epserde")]

use std::path::PathBuf;

use epserde::prelude::*;

use minhash_rs::index::{BandEntry, LshIndex, LshIndexRepr, NoStore};
use minhash_rs::prelude::*;

const P: usize = 128;
const BANDS: usize = 16;

fn deterministic_signature(seed: u64) -> MinHash<u64, P> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut out = MinHash::<u64, P>::new();
    for _ in 0..50 {
        state = state.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
        out.insert(state);
    }
    out
}

fn signatures(count: usize) -> Vec<MinHash<u64, P>> {
    (0..count as u64).map(deterministic_signature).collect()
}

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "minhash_rs_test_{}_{}.bin",
        name,
        std::process::id()
    ))
}

/// Two indices built from the same signatures must produce identical
/// candidate slices on the same query.
fn assert_candidates_match<K, const B: usize>(
    a: &LshIndex<K, P, B, NoStore>,
    b: &LshIndex<K, P, B, NoStore>,
    queries: &[K],
) where
    K: MinHasher<P>,
    K::Word: core::hash::Hash,
{
    let mut state_a = QueryState::new();
    let mut state_b = QueryState::new();
    for query in queries {
        let ca = a.candidates(query, &mut state_a).to_vec();
        let cb = b.candidates(query, &mut state_b).to_vec();
        assert_eq!(ca, cb, "candidate slices must match");
    }
}

#[test]
fn into_repr_then_from_repr_round_trip() {
    let sigs = signatures(64);
    let original: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let sibling: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());

    let repr = sibling.into_repr();
    let restored: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> = LshIndex::from_repr(repr);

    let queries = signatures(24);
    assert_candidates_match(&original, &restored, &queries);
}

#[test]
fn load_full_matches_in_memory() {
    let sigs = signatures(64);
    let in_memory: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let for_disk: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let repr = for_disk.into_repr();

    let path = tmp_path("load_full");
    let _ = std::fs::remove_file(&path);
    unsafe { repr.store(&path).expect("store") };

    let loaded_repr: LshIndexRepr<Vec<BandEntry>, BANDS> =
        unsafe { <LshIndexRepr<Vec<BandEntry>, BANDS>>::load_full(&path).expect("load_full") };
    let loaded: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> = LshIndex::from_repr(loaded_repr);

    let queries = signatures(24);
    assert_candidates_match(&in_memory, &loaded, &queries);

    let _ = std::fs::remove_file(&path);
}

/// Pin the zero-copy mmap contract. `LshIndexRepr<Vec<BandEntry>,
/// BANDS>::DeserType<'a>` must be `LshIndexRepr<&'a [BandEntry],
/// BANDS>`, so `loaded.band_entries` is a slice reference and not a
/// heap-owned `Vec`. This is enforced at the type level: the
/// `entries_is_slice_not_vec` binding below will fail to compile if the
/// derive ever regresses to full-copying the entries field.
#[test]
fn mmap_returns_true_zero_copy_slice_and_matches_in_memory() {
    let sigs = signatures(128);
    let in_memory: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let for_disk: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let repr = for_disk.into_repr();

    let path = tmp_path("mmap_zero_copy");
    let _ = std::fs::remove_file(&path);
    unsafe { repr.store(&path).expect("store") };

    let mem_case = unsafe {
        <LshIndexRepr<Vec<BandEntry>, BANDS>>::mmap(&path, Flags::empty()).expect("mmap")
    };
    let loaded = mem_case.uncase();

    // Type-level assertion: `loaded.band_entries` MUST be a `&[BandEntry]`.
    // If the epserde derive ever falls back to full-copying this field,
    // this binding fails to type-check and the test refuses to compile.
    let entries_is_slice_not_vec: &[BandEntry] = loaded.band_entries;
    assert_eq!(
        entries_is_slice_not_vec.len(),
        128 * BANDS,
        "entries slice length must equal signatures * bands"
    );

    // Query via the Repr's shared candidates method (works uniformly on
    // owned and mmap-loaded forms).
    let mut state_direct = QueryState::new();
    let mut state_mmap = QueryState::new();
    for query in signatures(48) {
        let direct = in_memory.candidates(&query, &mut state_direct).to_vec();
        let via_mmap = loaded.candidates::<_, P>(&query, &mut state_mmap).to_vec();
        assert_eq!(direct, via_mmap, "mmap query must match in-memory");
    }

    drop(mem_case);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn round_trip_empty_index() {
    let sigs: Vec<MinHash<u64, P>> = Vec::new();
    let original: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let sibling: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> = LshIndex::from_signatures(sigs);
    let restored: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_repr(sibling.into_repr());

    let queries = signatures(4);
    assert_candidates_match(&original, &restored, &queries);
}

#[test]
fn round_trip_single_signature_index() {
    let sigs = signatures(1);
    let original: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_signatures(sigs.clone());
    let sibling: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> = LshIndex::from_signatures(sigs);
    let restored: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
        LshIndex::from_repr(sibling.into_repr());

    let queries = signatures(4);
    assert_candidates_match(&original, &restored, &queries);
}

/// Exercise a different `(P, BANDS)` combination to catch off-by-one
/// errors in the band-offset arithmetic that a single-config test would
/// miss.
#[test]
fn round_trip_p64_bands8_full_disk_cycle() {
    const P2: usize = 64;
    const B2: usize = 8;

    fn sig(seed: u64) -> MinHash<u64, P2> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut out = MinHash::<u64, P2>::new();
        for _ in 0..32 {
            state = state.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1);
            out.insert(state);
        }
        out
    }

    let sigs: Vec<MinHash<u64, P2>> = (0..64u64).map(sig).collect();
    let in_memory: LshIndex<_, P2, B2, NoStore> = LshIndex::from_signatures(sigs.clone());
    let for_disk: LshIndex<_, P2, B2, NoStore> = LshIndex::from_signatures(sigs);

    let path = tmp_path("p64_b8_cycle");
    let _ = std::fs::remove_file(&path);
    unsafe { for_disk.into_repr().store(&path).expect("store") };

    let mem_case =
        unsafe { <LshIndexRepr<Vec<BandEntry>, B2>>::mmap(&path, Flags::empty()).expect("mmap") };
    let loaded = mem_case.uncase();

    // Same type-level pin at the second config.
    let entries_is_slice: &[BandEntry] = loaded.band_entries;
    assert!(
        !entries_is_slice.is_empty(),
        "entries slice must be nonempty"
    );

    let mut state_a = QueryState::new();
    let mut state_b = QueryState::new();
    for query_seed in 0..24u64 {
        let query = sig(query_seed);
        let direct = in_memory.candidates(&query, &mut state_a).to_vec();
        let via_mmap = loaded.candidates::<_, P2>(&query, &mut state_b).to_vec();
        assert_eq!(
            direct, via_mmap,
            "mmap must match in-memory at P=64, BANDS=8"
        );
    }

    drop(mem_case);
    let _ = std::fs::remove_file(&path);
}
