#![feature(test)]
extern crate test;

use minhash_rs::prelude::*;

use test::{black_box, Bencher};

#[bench]
fn bench_minhash_insert_with_siphashes13(b: &mut Bencher) {
    const NUMBER_OF_ELEMENTS: usize = 100_000;
    let mut hll: MinHash<u64, 128> = MinHash::new();

    b.iter(|| {
        // Inner closure, the actual test
        black_box(for i in 0..NUMBER_OF_ELEMENTS {
            hll.insert_with_siphashes13(i)
        });
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_siphashes13(b: &mut Bencher) {
    const NUMBER_OF_ELEMENTS: usize = 100_000;
    let mut hll: MinHash<u64, 128> = MinHash::new();
    let key0 = 0x0123456789ABCDEF;
    let key1 = 0xFEDCBA9876543210;

    b.iter(|| {
        // Inner closure, the actual test
        black_box(for i in 0..NUMBER_OF_ELEMENTS {
            hll.insert_with_keyed_siphashes13(i, key0, key1);
        });
    });
}

#[bench]
fn bench_minhash_insert_with_fvn(b: &mut Bencher) {
    const NUMBER_OF_ELEMENTS: usize = 100_000;
    let mut hll: MinHash<u64, 128> = MinHash::new();

    b.iter(|| {
        // Inner closure, the actual test
        black_box(for i in 0..NUMBER_OF_ELEMENTS {
            hll.insert_with_fvn(i)
        });
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_fvn(b: &mut Bencher) {
    const NUMBER_OF_ELEMENTS: usize = 100_000;
    let mut hll: MinHash<u64, 128> = MinHash::new();
    let key: u64 = 0x0123456789ABCDEF;

    b.iter(|| {
        // Inner closure, the actual test
        black_box(for i in 0..NUMBER_OF_ELEMENTS {
            hll.insert_with_keyed_fvn(i, key)
        });
    });
}
