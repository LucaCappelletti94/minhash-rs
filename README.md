# MinHash-rs
[![Build status](https://github.com/lucacappelletti94/minhash-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/lucacappelletti94/minhash-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/minhash-rs.svg)](https://crates.io/crates/minhash-rs)
[![Documentation](https://docs.rs/minhash-rs/badge.svg)](https://docs.rs/minhash-rs)

A Rust implementation of MinHash trying to be parsimonious with memory.

## What is MinHash?
MinHash is a probabilistic data structure used to estimate the similarity between two sets. It is based on the observation that if we hash two sets of objects, the probability that the hashes agree is equal to the Jaccard similarity between the two sets.

### How does it work?
MinHash works by hashing the elements of a set and keeping track of the minimum hash value for each hash function. The probability that the minimum hash value of two sets is the same is equal to the Jaccard similarity between the two sets. By using multiple hash functions, we can estimate the Jaccard similarity between two sets by averaging the probability that the minimum hash value of the two sets is the same.

![MinHash](https://github.com/LucaCappelletti94/minhash-rs/blob/main/minhash_diagram.jpg?raw=true)

## Using this crate
As usual, just add the following to your `Cargo.toml` file, altough remember to check out the benchmark results below before going for MinHash over [HyperLogLog](https://github.com/LucaCappelletti94/hyperloglog-rs).

```toml
[dependencies]
minhash-rs = "0.1.0"
```

## Reason for this implementation
I wanted to benchmark how well does MinHash estimates the Jaccard similarity between two sets and how well does it compare with other methods such as [HyperLogLog](https://github.com/LucaCappelletti94/hyperloglog-rs). The implementations I have found used more memory than it was necessary by the data structure, and I wanted to compare the performance of MinHash with other methods using the same amount of memory. Additionally, oftencase the methods were not optimized in any way shape or form, and I wanted to compare as fairly as possible MinHash with my rather well optimized implementation of HyperLogLog. I have benchmarked MinHash on many different universe sizes, [you can find the Jupyter Notebook here](https://github.com/LucaCappelletti94/minhash-rs/blob/main/MinHash%20Jaccard%20benchmarks.ipynb).

You can find the raw benchmark results in the [tests folder, the compressed CSVs](https://github.com/LucaCappelletti94/minhash-rs/tree/main/tests). I will keep adding datapoints for a while to be extra sure, but the results seem to be already quite clear.

**After days of benchmarking, it seems the results are in and indeed MinHash does not seem to be ever preferable to HyperLogLog at a comparable memory requirement.**

Here is the most comprehensive visualization ([find more in the Jupyter Notebook](https://github.com/LucaCappelletti94/minhash-rs/blob/main/MinHash%20Jaccard%20benchmarks.ipynb)), where the x axis is the memory in bits (log scale) and the y axis is the product of the MSE of estimating the Jaccard similarity multiplied by the time required to estimate the Jaccard similarity, also in log scale. The lower the better.

![MinHash HLL comparison](https://github.com/LucaCappelletti94/minhash-rs/blob/main/figures/minhash_hll_jaccard_MSE_x_time_and_memory.jpg?raw=true)