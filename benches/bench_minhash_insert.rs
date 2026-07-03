//! Insertion benchmarks for the non-atomic MinHash hash families.
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
            minhash.insert(black_box(i));
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_siphashes13(b: &mut Bencher) {
    let key0 = 0x0123_4567_89AB_CDEF;
    let key1 = 0xFEDC_BA98_7654_3210;

    b.iter(|| {
        let mut minhash: MinHash<u64, 128, SipHashes13Keyed> = MinHash::new_with_keys(key0, key1);
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert(black_box(i));
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_fnv(b: &mut Bencher) {
    b.iter(|| {
        let mut minhash: MinHash<u64, 128, Fnv> = MinHash::new();
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert(black_box(i));
        }
        black_box(minhash)
    });
}

#[bench]
fn bench_minhash_insert_with_keyed_fnv(b: &mut Bencher) {
    let key: u64 = 0x0123_4567_89AB_CDEF;

    b.iter(|| {
        let mut minhash: MinHash<u64, 128, FnvKeyed> = MinHash::new_with_keys(key, 0);
        for i in 0..NUMBER_OF_ELEMENTS {
            minhash.insert(black_box(i));
        }
        black_box(minhash)
    });
}
