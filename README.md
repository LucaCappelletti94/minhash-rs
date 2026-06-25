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
minhash-rs = "0.3.0"
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

### Locality-sensitive hashing (LSH)

Split a signature into bands to turn it into candidate keys for near-duplicate detection and similarity joins:

```rust
use minhash_rs::prelude::*;

let left: MinHash<u64, 128> = (0..100u64).collect();
let right: MinHash<u64, 128> = (0..100u64).collect();

let left_bands = left.band_hashes::<16>();
let right_bands = right.band_hashes::<16>();
assert!(left_bands.iter().zip(&right_bands).any(|(a, b)| a == b));
```

New benchmarks are under way.
