//! Criterion benchmarks comparing sparse and dense MinHash modes.
#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use minhash_rs::prelude::*;

const SMALL_SET: usize = 100;
const MEDIUM_SET: usize = 1_000;
const LARGE_SET: usize = 50_000;

// ─── Insert: sparse vs dense ────────────────────────────────────────────────

fn bench_insert_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert/sparse_vs_dense");
    group.sample_size(100);

    for &cardinality in &[SMALL_SET, MEDIUM_SET, LARGE_SET] {
        // siphashes13 (default hasher)
        group.bench_function(BenchmarkId::new("sparse_siphashes13", cardinality), |b| {
            b.iter_batched(
                MinHash::<u64, 128>::sparse,
                |mut mh| {
                    for i in 0..cardinality as u64 {
                        mh.insert(i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });

        group.bench_function(BenchmarkId::new("dense_siphashes13", cardinality), |b| {
            b.iter_batched(
                MinHash::<u64, 128>::new,
                |mut mh| {
                    for i in 0..cardinality as u64 {
                        mh.insert(i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });

        // fnv
        group.bench_function(BenchmarkId::new("sparse_fnv", cardinality), |b| {
            b.iter_batched(
                MinHash::<u64, 128, Fnv>::sparse,
                |mut mh| {
                    for i in 0..cardinality as u64 {
                        mh.insert(i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });

        group.bench_function(BenchmarkId::new("dense_fnv", cardinality), |b| {
            b.iter_batched(
                MinHash::<u64, 128, Fnv>::new,
                |mut mh| {
                    for i in 0..cardinality as u64 {
                        mh.insert(i);
                    }
                    mh
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

// ─── may_contain: sparse vs dense ───────────────────────────────────────────

fn bench_may_contain_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("may_contain/sparse_vs_dense");
    group.sample_size(100);

    for &cardinality in &[SMALL_SET, MEDIUM_SET] {
        let sparse: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                mh.insert(i);
                mh
            });
        let dense: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::new(), |mut mh, i| {
                mh.insert(i);
                mh
            });

        // Check present values
        group.bench_function(BenchmarkId::new("sparse_present", cardinality), |b| {
            b.iter(|| {
                for i in 0..cardinality as u64 {
                    let _ = sparse.may_contain(i);
                }
            });
        });

        group.bench_function(BenchmarkId::new("dense_present", cardinality), |b| {
            b.iter(|| {
                for i in 0..cardinality as u64 {
                    let _ = dense.may_contain(i);
                }
            });
        });

        // Check absent values
        group.bench_function(BenchmarkId::new("sparse_absent", cardinality), |b| {
            b.iter(|| {
                for i in cardinality as u64..(2 * cardinality) as u64 {
                    let _ = sparse.may_contain(i);
                }
            });
        });

        group.bench_function(BenchmarkId::new("dense_absent", cardinality), |b| {
            b.iter(|| {
                for i in cardinality as u64..(2 * cardinality) as u64 {
                    let _ = dense.may_contain(i);
                }
            });
        });
    }

    group.finish();
}

// ─── Jaccard: sparse vs dense ───────────────────────────────────────────────

fn bench_jaccard_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("jaccard/sparse_vs_dense");
    group.sample_size(100);

    for &cardinality in &[SMALL_SET, MEDIUM_SET] {
        let sparse_a: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                mh.insert(i);
                mh
            });
        let sparse_b: MinHash<u64, 128> = ((cardinality / 2) as u64..(cardinality * 3 / 2) as u64)
            .fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                mh.insert(i);
                mh
            });

        let dense_a: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::new(), |mut mh, i| {
                mh.insert(i);
                mh
            });
        let dense_b: MinHash<u64, 128> = ((cardinality / 2) as u64..(cardinality * 3 / 2) as u64)
            .fold(MinHash::<u64, 128>::new(), |mut mh, i| {
                mh.insert(i);
                mh
            });

        group.bench_function(BenchmarkId::new("sparse_vs_sparse", cardinality), |b| {
            b.iter(|| sparse_a.estimate_jaccard_index(&sparse_b));
        });

        group.bench_function(BenchmarkId::new("dense_vs_dense", cardinality), |b| {
            b.iter(|| dense_a.estimate_jaccard_index(&dense_b));
        });

        group.bench_function(BenchmarkId::new("sparse_vs_dense", cardinality), |b| {
            b.iter(|| sparse_a.estimate_jaccard_index(&dense_b));
        });
    }

    group.finish();
}

// ─── Union: sparse vs dense ─────────────────────────────────────────────────

fn bench_union_sparse_vs_dense(c: &mut Criterion) {
    let mut group = c.benchmark_group("union/sparse_vs_dense");
    group.sample_size(100);

    for &cardinality in &[SMALL_SET, MEDIUM_SET] {
        let sparse_a: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                mh.insert(i);
                mh
            });
        let sparse_b: MinHash<u64, 128> = ((cardinality / 2) as u64..(cardinality * 3 / 2) as u64)
            .fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                mh.insert(i);
                mh
            });

        let dense_a: MinHash<u64, 128> =
            (0..cardinality as u64).fold(MinHash::<u64, 128>::new(), |mut mh, i| {
                mh.insert(i);
                mh
            });
        let dense_b: MinHash<u64, 128> = ((cardinality / 2) as u64..(cardinality * 3 / 2) as u64)
            .fold(MinHash::<u64, 128>::new(), |mut mh, i| {
                mh.insert(i);
                mh
            });

        group.bench_function(BenchmarkId::new("sparse_vs_sparse", cardinality), |b| {
            b.iter_batched(
                || (sparse_a, sparse_b),
                |(mut a, b)| {
                    a |= &b;
                    a
                },
                BatchSize::PerIteration,
            );
        });

        group.bench_function(BenchmarkId::new("dense_vs_dense", cardinality), |b| {
            b.iter_batched(
                || (dense_a, dense_b),
                |(mut a, b)| {
                    a |= &b;
                    a
                },
                BatchSize::PerIteration,
            );
        });

        group.bench_function(BenchmarkId::new("sparse_vs_dense", cardinality), |b| {
            b.iter_batched(
                || (sparse_a, dense_b),
                |(mut a, b)| {
                    a |= &b;
                    a
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

// ─── Densification cost ─────────────────────────────────────────────────────

fn bench_densification(c: &mut Criterion) {
    let mut group = c.benchmark_group("densification");
    group.sample_size(100);

    // Build a sparse sketch near capacity, then measure auto-densification
    // (triggered by overflow insert)
    let near_capacity: u64 = 127; // PERMUTATIONS - 1 for 128

    group.bench_function("auto_densify_128", |b| {
        b.iter_batched(
            || {
                let mut mh = MinHash::<u64, 128>::sparse();
                for i in 0..near_capacity {
                    mh.insert(i);
                }
                mh
            },
            |mut mh| {
                // This insert triggers densification
                mh.insert(999_999);
                mh
            },
            BatchSize::PerIteration,
        );
    });

    // Measure densification via equality check (sparse vs dense)
    let sparse: MinHash<u64, 128> =
        (0..MEDIUM_SET as u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
            mh.insert(i);
            mh
        });
    let dense: MinHash<u64, 128> = MinHash::<u64, 128>::new();

    group.bench_function("densify_via_eq", |b| {
        b.iter(|| sparse == dense);
    });

    group.finish();
}

// ─── band_hashes: sparse vs dense ───────────────────────────────────────────

fn bench_band_hashes(c: &mut Criterion) {
    let mut group = c.benchmark_group("band_hashes");
    group.sample_size(100);

    let sparse: MinHash<u64, 128> =
        (0..MEDIUM_SET as u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
            mh.insert(i);
            mh
        });
    let dense: MinHash<u64, 128> =
        (0..MEDIUM_SET as u64).fold(MinHash::<u64, 128>::new(), |mut mh, i| {
            mh.insert(i);
            mh
        });

    group.bench_function("sparse", |b| b.iter(|| sparse.band_hashes::<16>()));
    group.bench_function("dense", |b| b.iter(|| dense.band_hashes::<16>()));

    group.finish();
}

// ─── is_empty / is_full ─────────────────────────────────────────────────────

fn bench_state_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_checks");
    group.sample_size(100);

    let sparse_empty = MinHash::<u64, 128>::sparse();
    let dense_empty = MinHash::<u64, 128>::new();

    let sparse_full: MinHash<u64, 128> =
        (0..200_u64).fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
            mh.insert(i);
            mh
        });
    let dense_populated: MinHash<u64, 128> =
        (0..200_u64).fold(MinHash::<u64, 128>::new(), |mut mh, i| {
            mh.insert(i);
            mh
        });

    group.bench_function("sparse_is_empty", |b| b.iter(|| sparse_empty.is_empty()));
    group.bench_function("dense_is_empty", |b| b.iter(|| dense_empty.is_empty()));
    group.bench_function("sparse_is_full", |b| b.iter(|| sparse_full.is_full()));
    group.bench_function("dense_is_full", |b| b.iter(|| dense_populated.is_full()));

    group.finish();
}

// ─── FromIterator ───────────────────────────────────────────────────────────

fn bench_from_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_iter");
    group.sample_size(100);

    for &cardinality in &[SMALL_SET, MEDIUM_SET, LARGE_SET] {
        group.bench_function(BenchmarkId::new("collect_dense", cardinality), |b| {
            b.iter_batched(
                || 0..cardinality as u64,
                MinHash::<u64, 128>::from_iter,
                BatchSize::PerIteration,
            );
        });

        group.bench_function(BenchmarkId::new("loop_sparse", cardinality), |b| {
            b.iter_batched(
                || 0..cardinality as u64,
                |range| {
                    range.fold(MinHash::<u64, 128>::sparse(), |mut mh, i| {
                        mh.insert(i);
                        mh
                    })
                },
                BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_sparse_vs_dense,
    bench_may_contain_sparse_vs_dense,
    bench_jaccard_sparse_vs_dense,
    bench_union_sparse_vs_dense,
    bench_densification,
    bench_band_hashes,
    bench_state_checks,
    bench_from_iter,
);
criterion_main!(benches);
