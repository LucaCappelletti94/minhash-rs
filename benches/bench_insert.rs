//! Criterion benchmarks for MinHash insertion performance.
//!
//! Covers cold vs. mature sketches, multiple permutation counts, word widths,
//! and hash families to measure the impact of internal optimizations.
#![allow(missing_docs)]

use core::sync::atomic::Ordering;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use minhash_rs::prelude::*;

const ELEMENTS: usize = 10_000;

fn bench_insert_families(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/families");
    group.sample_size(100);

    for (name, f) in [
        (
            "siphashes13",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_siphashes13(i);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
        (
            "keyed_siphashes13",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_keyed_siphashes13(i, 0x0123_4567_89AB_CDEF, 0xFEDC_BA98_7654_3210);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
        (
            "fnv",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_fnv(i);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
        (
            "keyed_fnv",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_keyed_fnv(i, 0x0123_4567_89AB_CDEF);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
    ] {
        group.bench_function(BenchmarkId::new("cold", name), |b| {
            b.iter_batched(
                MinHash::<u64, 128>::new,
                |mut mh| {
                    for i in 0..ELEMENTS as u64 {
                        f(&mut mh, i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

fn bench_insert_mature(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/mature");
    group.sample_size(100);

    // Pre-populate a sketch so most inserts will not improve any register.
    let mut warm = MinHash::<u64, 128>::new();
    for i in 0..1_000_000u64 {
        warm.insert_with_siphashes13(i);
    }

    for (name, f) in [
        (
            "siphashes13",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_siphashes13(i);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
        (
            "fnv",
            Box::new(|mh: &mut MinHash<u64, 128>, i: u64| {
                mh.insert_with_fnv(i);
            }) as Box<dyn Fn(&mut MinHash<u64, 128>, u64)>,
        ),
    ] {
        group.bench_function(name, |b| {
            b.iter_batched(
                || warm,
                |mut mh| {
                    for i in 0..ELEMENTS as u64 {
                        f(&mut mh, i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

fn bench_insert_permutations(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/permutations");
    group.sample_size(100);

    // u64 with varying permutation counts
    group.bench_function(BenchmarkId::new("u64", 16), |b| {
        b.iter_batched(
            MinHash::<u64, 16>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function(BenchmarkId::new("u64", 128), |b| {
        b.iter_batched(
            MinHash::<u64, 128>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function(BenchmarkId::new("u64", 1024), |b| {
        b.iter_batched(
            MinHash::<u64, 1024>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function(BenchmarkId::new("u64", 8192), |b| {
        b.iter_batched(
            MinHash::<u64, 8192>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn bench_insert_word_widths(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/word_widths");
    group.sample_size(100);

    group.bench_function("u8", |b| {
        b.iter_batched(
            MinHash::<u8, 128>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("u16", |b| {
        b.iter_batched(
            MinHash::<u16, 128>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("u32", |b| {
        b.iter_batched(
            MinHash::<u32, 128>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.bench_function("u64", |b| {
        b.iter_batched(
            MinHash::<u64, 128>::new,
            |mut mh| {
                for i in 0..ELEMENTS as u64 {
                    mh.insert_with_fnv(i);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn bench_insert_atomic(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/atomic");
    group.sample_size(100);

    group.bench_function("fetch_min_fnv", |b| {
        b.iter_batched(
            MinHash::<u64, 128>::new,
            |mut mh| {
                let atomic = mh.as_atomic();
                for i in 0..ELEMENTS as u64 {
                    atomic.fetch_insert_with_fnv(i, Ordering::Relaxed);
                }
                mh
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_families,
    bench_insert_mature,
    bench_insert_permutations,
    bench_insert_word_widths,
    bench_insert_atomic,
);
criterion_main!(benches);
