#![feature(test)]
extern crate test;

use minhash_rs::prelude::*;

use test::{black_box, Bencher};

const NUMBER_OF_ELEMENTS: usize = 100_000;

#[bench]
fn bench_minhash_insert_with_siphashes13(b: &mut Bencher) {
    b.iter(|| {
        let mut minhash: MinHash<u64, 128> = MinHash::new();
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert_with_siphashes13(black_box(i));
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_siphashes13(b: &mut Bencher) {
    let key0 = 0x0123456789ABCDEF;
    let key1 = 0xFEDCBA9876543210;

    b.iter(|| {
        let mut minhash: MinHash<u64, 128> = MinHash::new();
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert_with_keyed_siphashes13(black_box(i), key0, key1);
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_fnv(b: &mut Bencher) {
    b.iter(|| {
        let mut minhash: MinHash<u64, 128> = MinHash::new();
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert_with_fnv(black_box(i));
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_fnv(b: &mut Bencher) {
    let key: u64 = 0x0123456789ABCDEF;

    b.iter(|| {
        let mut minhash: MinHash<u64, 128> = MinHash::new();
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert_with_keyed_fnv(black_box(i), key);
        }
        black_box(minhash)
    });
}
