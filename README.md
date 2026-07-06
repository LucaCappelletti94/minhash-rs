# MinHash-rs

[![Build status](https://github.com/lucacappelletti94/minhash-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/lucacappelletti94/minhash-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/minhash-rs.svg)](https://crates.io/crates/minhash-rs)
[![Documentation](https://docs.rs/minhash-rs/badge.svg)](https://docs.rs/minhash-rs)
[![codecov](https://codecov.io/gh/LucaCappelletti94/minhash-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/minhash-rs)

A Rust implementation of MinHash trying to be parsimonious with memory.

## What is MinHash?

MinHash is a probabilistic data structure used to estimate the similarity between two sets. It is based on the observation that if we hash two sets of objects, the probability that the hashes agree is equal to the Jaccard similarity between the two sets.

### How does it work?

MinHash works by hashing the elements of a set and keeping track of the minimum hash value for each hash function. The probability that the minimum hash value of two sets is the same is equal to the Jaccard similarity between the two sets. By using multiple hash functions, we can estimate the Jaccard similarity between two sets by averaging the probability that the minimum hash value of the two sets is the same.

![MinHash](https://github.com/LucaCappelletti94/minhash-rs/blob/main/minhash_diagram.jpg?raw=true)

## Using this crate

As usual, just add the following to your `Cargo.toml` file, although remember to check out the benchmark results below before going for MinHash over [HyperLogLog](https://github.com/LucaCappelletti94/hyperloglog-rs).

```toml
[dependencies]
minhash-rs = "0.5.0"
```

### Example

Build a sketch from each set and estimate their Jaccard similarity:

```rust
use std::collections::HashSet;
use minhash_rs::prelude::*;

let left: HashSet<u64> = (0..100).collect();
let right: HashSet<u64> = (50..150).collect();

// A MinHash over `u64` words with 256 permutations.
let left_sketch: MinHash<u64, 256> = left.iter().copied().collect();
let right_sketch: MinHash<u64, 256> = right.iter().copied().collect();

let estimate = left_sketch.estimate_jaccard_index(&right_sketch);

// The true Jaccard index here is 50 / 150 = 1/3.
let truth = 1.0 / 3.0;
assert!((estimate - truth).abs() < 0.1);

// Sketches can also be merged: `a | b` is the sketch of the union of the sets.
let union_sketch = left_sketch | right_sketch;
let union: HashSet<u64> = left.union(&right).copied().collect();
assert_eq!(union_sketch, union.iter().copied().collect());
```

### Sparse prefixes

Two wrapper types add a sparse prefix that promotes to `MinHash` at capacity. Both hold an inner `MinHash` and share the state-equivalence invariant: after promotion the inner signature is bit-identical to what a from-scratch `MinHash` on the same input would have produced, so classical banded LSH via `band_hashes::<BANDS>()` keeps working across the transition. Both wrappers implement `MinHasher<PERMUTATIONS>`, so `MinHash`, `SparseHashes`, and `SparseValues` are interchangeable behind trait bounds. The trait's `estimate_jaccard_index` compares two sketches of the same variant and takes the exact bottom-`P` merge whenever both sides are still under capacity. Cross-variant Jaccard (e.g. `SparseHashes` against `SparseValues`) is done explicitly by converting both operands to `MinHash` with `MinHash::from(sketch)` or `sketch.into()` and comparing the resulting dense sketches.

#### SparseHashes

`SparseHashes<Word, PERMUTATIONS>` stores hash digests in a sorted bottom-k list and defers permutation expansion until the buffer overflows. The stored digests come from the same `Hasher` the eventual dense `MinHash` uses, so no elements are rehashed at promotion. Reach for this variant when the input elements are arbitrary payloads and the hash of the element is what matters, or when the value domain is not compressible enough for a value codec to earn its keep.

```rust
use minhash_rs::prelude::*;

let mut sketch = SparseHashes::<u64, 128>::new();
sketch.insert(42u64);
assert!(sketch.may_contain(42u64));

// Consume the wrapper for the dense signature. Classical LSH lives on `MinHash`.
let dense: MinHash<u64, 128> = sketch.into();
assert!(dense.may_contain(42u64));
```

#### SparseValues

`SparseValues<PERMUTATIONS, Hasher, Hash, Code>` accepts raw `u64` values (the caller casts if their integer domain fits in `u64`) and stores them under a `dsi-bitstream` instantaneous code chosen at the type level, defaulting to `ConstCode<{ code_consts::GAMMA }>`. Any code shipped by `dsi-bitstream` (delta, omega, Rice, exponential Golomb, zeta, pi) can be swapped in by naming it as `Code`. Compressible integer domains such as dense ranges or topologically ordered graph node identifiers collapse to roughly the codeword length per stored value, which is the reason to reach for this variant over `SparseHashes` when the input is already a compact integer identifier.

```rust
use minhash_rs::prelude::*;

let mut sketch = SparseValues::<128>::new();
for id in 0u64..100 {
    sketch.insert(id);
}
assert!(sketch.may_contain(42u64));
let dense: MinHash<u64, 128> = sketch.into();
```

#### Sparse-mode semantics

Sparse-mode Jaccard is exact on the retained set (digests for `SparseHashes`, raw values for `SparseValues`), not on the original input elements: ordinary hash collisions and (for `SparseHashes`) the `saturating_add(1)` encoding collision at `Hash::MAX` still count. Densification is a deterministic latency spike on the specific insert that triggers overflow, paying `O(PERMUTATIONS * PERMUTATIONS)` on that record before every subsequent insert returns to the from-scratch dense cost of `O(PERMUTATIONS)`. Real-time streaming callers who need smooth tail latency should either pre-densify with `MinHash::new()` or budget the spike explicitly.

### LSH indexing

The `alloc` feature (on by default) exposes `LshIndex`, a compile-time banded LSH index over any `MinHasher`. The index stores every band's entries in one flat `Vec<BandEntry>` sorted per band, plus a `[u32; BANDS]` array of per-band start offsets, and returns candidate ids ranked by descending collision count, so callers can cheaply cap the refine pass at whatever depth the S-curve demands.

```rust
use minhash_rs::prelude::*;

let a: MinHash<u64, 128> = (0u64..30).collect();
let b: MinHash<u64, 128> = (15u64..45).collect();
let c: MinHash<u64, 128> = (200u64..230).collect();

let index: LshIndex<MinHash<u64, 128>, 128, 16, Store> =
    LshIndex::from_signatures([a, b, c]);

let mut state = QueryState::new();
let hits = index.top_k(&a, 2, &mut state);
assert_eq!(hits[0].0, 0);
assert!((hits[0].1 - 1.0).abs() < 1e-9);
```

Type-level signature storage. `LshIndex<K, P, BANDS, Store>` keeps a `Vec<K>` alongside the band tables and unlocks `signature(id)` and `top_k(query, k, state)`. `LshIndex<K, P, BANDS, NoStore>` (the default) drops signatures after computing their band hashes and leaves refinement to the caller, which is what production builds at 10M-plus want since the signature `Vec` dominates memory well before the band tables do. The marker is a zero-sized type parameter, so the two variants share the same runtime code path apart from the storage cost.

Zero-alloc query hot path. `candidates` and `top_k` take a caller-owned `QueryState` scratch. Reusing the same state across many queries pins the internal buffers to their steady-state size and removes all per-query heap allocation. Compile-time `BANDS` means the per-band loop unrolls and the band-tables are a fixed-size `[u32; BANDS]` of start offsets over a single contiguous `Vec<BandEntry>`, so build touches one heap allocation instead of one per band.

#### Sparse mode and LSH recall

Candidate generation is unaffected by the mode of either operand. `SparseHashes` and `SparseValues` implement `MinHasher` with `band_hashes` on the sparse types building a dense `MinHash` via `From` first, then hashing the bands, so a sparse-mode sketch's query-time band hashes are bit-identical to those of a fresh dense `MinHash` on the same input. The promotion invariant the paper's design turns on is exactly what makes this work. Densification per query is `O(count * PERMUTATIONS)` with `count` bounded by `PERMUTATIONS`, so its cost is dominated by the LSH lookup itself. Callers who batch many queries against one sketch should densify once with `sketch.densify()` and reuse the result.

Where sparse mode changes retrieval quality is the refine step. `LshIndex::top_k` on the `Store` variant scores every returned candidate with `MinHasher::estimate_jaccard_index` before ranking, and that estimator has an exact fast path for sparse-sparse pairs of the same variant. When both the query and the stored candidate are still sparse, refinement runs as a sorted-list intersection over the retained sets and returns the true set Jaccard rather than the classical MinHash estimator with its `Θ(J * (1 - J) / PERMUTATIONS)` variance. Sparse-value operands are exact on the original input elements. Sparse-hash operands are exact on the digest set subject to the underlying hasher's `1 / 2^N` collision floor. Refined scores therefore carry no register noise on any sparse-sparse pair, the pipeline pays this fast path per pair rather than per query, and precision at `k` and recall both improve for those pairs, most in the small-`N` regime where sparse mode is profitable in the first place.

#### Parallel bulk build

Enable the `rayon` feature to unlock `LshIndex::from_signatures_par`, a Rayon-parallel counterpart to `from_signatures`. Every signature's `band_hashes` runs in parallel, each per-band slice sorts in parallel, and id assignment is preserved so the parallel build produces byte-for-byte the same band tables as the sequential build. On a Threadripper 5975WX the parallel path reaches roughly `4.8x` wall-time speedup at 1M signatures with `P = 128, BANDS = 16`. The sequential path stays the right pick on small inputs and remains the default.

#### Persistence via epserde

Enable the `epserde` feature to unlock `LshIndexRepr<E, BANDS>`, a persistent representation of the CSR band tables and signature count. The entries container is a generic parameter so `epserde`'s derive picks the zero-copy path for it: the owned form used at save time is `LshIndexRepr<Vec<BandEntry>, BANDS>`, and `mmap` returns `LshIndexRepr<&[BandEntry], BANDS>` where the entries slice points directly into the mapped file. `LshIndex::into_repr` and `LshIndex::from_repr` handle the owned round-trip on the `NoStore` variant. `LshIndexRepr::candidates` is available on both forms, so the same query code drives an in-memory index and a mmap-loaded index. At scale the mmap-loaded form only pages in the entries the query touches, not the entire index.

End-to-end round trip: build an index, save it, mmap-load it, run a query. This example is exercised as a doctest.

```rust
use epserde::prelude::*;
use minhash_rs::index::{BandEntry, LshIndex, LshIndexRepr, NoStore};
use minhash_rs::prelude::*;

const P: usize = 128;
const BANDS: usize = 16;

// A unique path under the OS temp dir so parallel doctests never collide.
let path = std::env::temp_dir()
    .join(format!("minhash_rs_readme_epserde_{}.bin", std::process::id()));

// Build a small index and serialise it.
let signatures: Vec<MinHash<u64, P>> = (0..1_000_u64)
    .map(|i| {
        let mut s = MinHash::new();
        s.insert(i);
        s
    })
    .collect();
let index: LshIndex<MinHash<u64, P>, P, BANDS, NoStore> =
    LshIndex::from_signatures(signatures);
let repr = index.into_repr();
unsafe { repr.store(&path)? };

// Load via mmap. `loaded.band_entries` is `&[BandEntry]` served straight
// from the file mapping, not a heap copy.
let mem_case = unsafe {
    <LshIndexRepr<Vec<BandEntry>, BANDS>>::mmap(&path, Flags::empty())?
};
let loaded = mem_case.uncase();

// Same `candidates` method as the in-memory index.
let mut query = MinHash::<u64, P>::new();
query.insert(42_u64);
let mut state = QueryState::new();
let hits: Vec<_> = loaded.candidates::<_, P>(&query, &mut state).to_vec();
assert!(!hits.is_empty(), "expected at least the self-match for signature 42");
assert!(hits.iter().any(|c| c.id == 42), "signature 42 must self-match");

drop(mem_case);
let _ = std::fs::remove_file(&path);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`load_full` is also available when you want the entries owned in a fresh `Vec` instead of served from the mapping.
