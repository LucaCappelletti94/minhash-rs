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
let left_sketch: MinHash<u64, 256> = left.iter().collect();
let right_sketch: MinHash<u64, 256> = right.iter().collect();

let estimate = left_sketch.estimate_jaccard_index(&right_sketch);

// The true Jaccard index here is 50 / 150 = 1/3.
let truth = 1.0 / 3.0;
assert!((estimate - truth).abs() < 0.1);

// Sketches can also be merged: `a | b` is the sketch of the union of the sets.
let union_sketch = left_sketch | right_sketch;
let union: HashSet<u64> = left.union(&right).copied().collect();
assert_eq!(union_sketch, union.iter().collect());
```

### Sparse prefixes

Two wrapper types add a sparse prefix that promotes to `MinHash` at capacity. Both hold an inner `MinHash` and share the state-equivalence invariant: after promotion the inner signature is bit-identical to what a from-scratch `MinHash` on the same input would have produced, so classical banded LSH via `band_hashes::<BANDS>()` keeps working across the transition. Both wrappers implement `MinHasher<PERMUTATIONS>`, so `MinHash`, `SparseHashes`, and `SparseValues` are interchangeable behind trait bounds. The trait's `estimate_jaccard_index` compares two sketches of the same variant and takes the exact bottom-`P` merge whenever both sides are still under capacity. Cross-variant Jaccard (e.g. `SparseHashes` against `SparseValues`) is done explicitly by calling `to_dense` on both operands first and comparing the resulting dense sketches.

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
