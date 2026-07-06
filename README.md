# MinHash-rs

[![Build status](https://github.com/lucacappelletti94/minhash-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/lucacappelletti94/minhash-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/minhash-rs.svg)](https://crates.io/crates/minhash-rs)
[![Documentation](https://docs.rs/minhash-rs/badge.svg)](https://docs.rs/minhash-rs)
[![codecov](https://codecov.io/gh/LucaCappelletti94/minhash-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/minhash-rs)

Parsimonious-memory MinHash. Sketches implement `MinHasher<PERMUTATIONS>`: a dense `MinHash<Word, PERMUTATIONS>` and two sparse wrappers `SparseHashes` and `SparseValues` that promote to dense at capacity. The promoted state is bit-identical to a from-scratch dense sketch on the same input, so classical banded LSH keeps working across the transition.

![MinHash](https://github.com/LucaCappelletti94/minhash-rs/blob/main/minhash_diagram.jpg?raw=true)

## Using this crate

```toml
[dependencies]
minhash-rs = "0.5.0"
```

Estimate Jaccard between two sets and take a union:

```rust
use std::collections::HashSet;
use minhash_rs::prelude::*;

let left: HashSet<u64> = (0..100).collect();
let right: HashSet<u64> = (50..150).collect();

let left_sketch: MinHash<u64, 256> = left.iter().copied().collect();
let right_sketch: MinHash<u64, 256> = right.iter().copied().collect();

let estimate = left_sketch.estimate_jaccard_index(&right_sketch);
let truth = 1.0 / 3.0;
assert!((estimate - truth).abs() < 0.1);

let union_sketch = left_sketch | right_sketch;
let union: HashSet<u64> = left.union(&right).copied().collect();
assert_eq!(union_sketch, union.iter().copied().collect());
```

### SparseHashes

`SparseHashes<Word, PERMUTATIONS>` stores hash digests in a sorted bottom-k list and defers permutation expansion until overflow. Reach for it when input elements are arbitrary payloads.

```rust
use minhash_rs::prelude::*;

let mut sketch = SparseHashes::<u64, 128>::new();
sketch.insert(42u64);
assert!(sketch.may_contain(42u64));

let dense: MinHash<u64, 128> = sketch.into();
assert!(dense.may_contain(42u64));
```

### SparseValues

`SparseValues<PERMUTATIONS, Hasher, Hash, Code>` stores raw `u64` values under a `dsi-bitstream` code, default `ConstCode<{ code_consts::GAMMA }>`. Compressible integer domains (dense ranges, topologically ordered graph ids) collapse to roughly the codeword length per stored value.

```rust
use minhash_rs::prelude::*;

let mut sketch = SparseValues::<128>::new();
for id in 0u64..100 {
    sketch.insert(id);
}
assert!(sketch.may_contain(42u64));
let dense: MinHash<u64, 128> = sketch.into();
```

Sparse-mode Jaccard is exact on the retained set (digests or values), not on the raw input elements. Densification is a `O(PERMUTATIONS * PERMUTATIONS)` spike on the specific insert that overflows.

## LSH indexing

The `alloc` feature exposes `LshIndex`, a compile-time banded index over any `MinHasher`. Band tables live in one flat `Vec<BandEntry>` plus a `[u32; BANDS]` of per-band start offsets. Candidates are ranked by descending collision count.

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

`Store` keeps the signatures and unlocks `signature(id)` and `top_k`. `NoStore` (the default) drops them after computing band hashes. `QueryState` is caller-owned scratch and removes all per-query heap allocation across a batch.

### Sparse mode changes refine, not candidates

Band hashes are bit-identical between dense and sparse variants, so candidate sets match by construction. `LshIndex::top_k` then scores each candidate via `MinHasher::estimate_jaccard_index`. Dense scores through the register-agreement estimator with `Θ(J * (1 - J) / PERMUTATIONS)` variance. Sparse-sparse pairs take an exact set-intersection fast path over the retained digests or values.

`examples/bench_sparse_lsh_recall.rs` runs 5000 chain-generated documents (cardinality 50, universe 512, both variants stay under the promotion gate) through both backends. Same candidates, different refine.

![Sparse vs dense LSH refine, recall and precision at threshold](https://github.com/LucaCappelletti94/minhash-rs/blob/main/figures/sparse_lsh_recall.png?raw=true)

Sparse holds precision `1.0` at every threshold. Dense drops to `0.89` precision at `τ = 0.9` because the `1 / sqrt(P) ≈ 0.088` estimator noise lifts non-neighbours above the cutoff. Recall separates from `τ = 0.7` upward: at `τ = 0.9` sparse recovers `1.00`, dense recovers `0.93`.

### Parallel build

The `rayon` feature enables `LshIndex::from_signatures_par`, byte-identical to `from_signatures` and roughly `4.8x` faster at 1M signatures on a Threadripper 5975WX with `P = 128, BANDS = 16`.

### Persistence via epserde

The `epserde` feature exposes `LshIndexRepr<E, BANDS>`. Owned form `E = Vec<BandEntry>` is written to disk. On `mmap`, the deserialised form is `LshIndexRepr<&[BandEntry], BANDS>` with the entries slice pointing directly into the mapped file. `LshIndexRepr::candidates` works uniformly on both forms.

```rust
use epserde::prelude::*;
use minhash_rs::index::{BandEntry, LshIndex, LshIndexRepr, NoStore};
use minhash_rs::prelude::*;

const P: usize = 128;
const BANDS: usize = 16;

let path = std::env::temp_dir()
    .join(format!("minhash_rs_readme_epserde_{}.bin", std::process::id()));

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

let mem_case = unsafe {
    <LshIndexRepr<Vec<BandEntry>, BANDS>>::mmap(&path, Flags::empty())?
};
let loaded = mem_case.uncase();

let mut query = MinHash::<u64, P>::new();
query.insert(42_u64);
let mut state = QueryState::new();
let hits: Vec<_> = loaded.candidates::<_, P>(&query, &mut state).to_vec();
assert!(!hits.is_empty());
assert!(hits.iter().any(|c| c.id == 42));

drop(mem_case);
let _ = std::fs::remove_file(&path);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`load_full` returns the entries owned in a fresh `Vec` instead of served from the mapping.
