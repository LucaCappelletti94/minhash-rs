#![feature(test)]
extern crate test;

use minhash_rs::prelude::*;

use test::{black_box, Bencher};

#[bench]
fn bench_minhash_insert(b: &mut Bencher) {
    const NUMBER_OF_ELEMENTS: usize = 100_000;
    let mut hll: MinHash<u64, 128> = MinHash::new();

    b.iter(|| {
        // Inner closure, the actual test

        black_box(for i in 0..NUMBER_OF_ELEMENTS {
            hll.insert(i)
        });
    });
}
