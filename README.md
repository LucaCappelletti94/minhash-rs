# MinHash-rs
[![Build status](https://github.com/lucacappelletti94/minhash-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/lucacappelletti94/minhash-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/minhash-rs.svg)](https://crates.io/crates/minhash-rs)
[![Documentation](https://docs.rs/minhash-rs/badge.svg)](https://docs.rs/minhash-rs)

A Rust implementation of MinHash trying to be parsimonious with memory.

## What is MinHash?
MinHash is a probabilistic data structure used to estimate the similarity between two sets. It is based on the observation that if we hash two sets of objects, the probability that the hashes agree is equal to the Jaccard similarity between the two sets.

## Reason for this implementation
I wanted to benchmark how well does MinHash estimates the Jaccard similarity between two sets and how well does it compare with other methods such as [HyperLogLog](https://github.com/LucaCappelletti94/hyperloglog-rs). The implementations I have found used more memory than it was necessary by the data structure, and I wanted to compare the performance of MinHash with other methods using the same amount of memory. Additionally, oftencase the methods were not optimized in any way shape or form, and I wanted to compare as fairly as possible MinHash with my rather well optimized implementation of HyperLogLog. I have benchmarked MinHash on many different universe sizes, [you can find the Jupyter Notebook here](https://github.com/LucaCappelletti94/minhash-rs/blob/main/MinHash%20Jaccard%20benchmarks.ipynb).

**After days of benchmarking, it seems the results are in and indeed MinHash does not seem to be ever preferable to HyperLogLog at a comparable memory requirement.**

Here is the most comprehensive visualization ([find more in the Jupyter Notebook](https://github.com/LucaCappelletti94/minhash-rs/blob/main/MinHash%20Jaccard%20benchmarks.ipynb)):