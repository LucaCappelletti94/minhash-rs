//! Compare LSH refine recall and precision between dense `MinHash` and
//! `SparseHashes` on the same signatures.
//!
//! The two backends produce bit-identical LSH candidates by construction,
//! so any difference in the refined output comes from the refine step
//! itself. Dense `MinHash::estimate_jaccard_index` is the classical
//! register-agreement estimator with roughly `1 / sqrt(P)` standard error.
//! `SparseHashes::estimate_jaccard_index`, when both operands are still
//! sparse, returns the exact set Jaccard of the retained digests.
//!
//! # Setup
//!
//! `P = 128`, `BANDS = 16`. Documents are generated as a smooth-mutation
//! chain: `doc[0]` is a random 50-element subset of a small universe
//! (`UNIVERSE = 512`), and each subsequent `doc[i]` is `doc[i - 1]` with a
//! single element substitution. The Jaccard between `doc[i]` and `doc[j]`
//! decays smoothly with `|i - j|`, so pair Jaccards sit at every level in
//! `(0, 1]` rather than at a handful of discrete cluster values. That
//! matches what fingerprint or spectral workloads look like in practice:
//! many pairs land near any given refine threshold, and the dense
//! estimator noise flips a big population.
//!
//! Cardinality 50 stays well under the `P / 2 = 64` sparse-mode promotion
//! gate, so every signature keeps its sparse representation and every
//! sparse-sparse refine call takes the exact set intersection fast path.
//! Universe 512 gives unrelated documents a nonzero baseline Jaccard
//! (`~ 0.05`) similar to a folded fingerprint bit vector.
//!
//! # Metrics
//!
//! For each query and threshold `tau`:
//!
//! - `refine_recall = |refined ∩ truth| / |candidates ∩ truth|`. Fraction of
//!   candidate-visible true positives kept by the refine step. Sparse hits
//!   `1.0` by construction. Dense loses candidates whose true Jaccard sits
//!   just above `tau` and gets pushed below by estimator noise.
//! - `precision = |refined ∩ truth| / |refined|`. Fraction of refined
//!   output that is a real true positive. Sparse hits `1.0` by construction.
//!   Dense keeps some candidates whose true Jaccard is below `tau` but
//!   whose noisy estimate crossed above it.
//! - `pipeline_recall = |refined ∩ truth| / |truth|`. End-to-end recall
//!   against absolute truth. Dominated by the shared LSH candidate step
//!   ceiling and identical across backends. Kept in the CSV for context.
//!
//! Output: one CSV line per `(threshold, backend)` pair on stdout.

use std::collections::HashSet;

use minhash_rs::index::{LshIndex, QueryState, Store};
use minhash_rs::prelude::*;

const P: usize = 128;
const BANDS: usize = 16;
const N_DOCS: usize = 5_000;
const CARDINALITY: usize = 50;
const UNIVERSE: u64 = 512;
const N_QUERIES: usize = 1_000;
const SEED: u64 = 0xDEAD_BEEF_CAFE_F00D;

/// Deterministic SplitMix64 so runs are reproducible across machines.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, hi: u64) -> u64 {
        self.next_u64() % hi
    }
}

/// Build a chain of `N_DOCS` documents. `doc[0]` is a random
/// `CARDINALITY`-element subset of the universe. `doc[i]` is `doc[i - 1]`
/// with one random element replaced by a random universe element not
/// already in the doc. The chain produces smoothly decaying Jaccard as
/// `|i - j|` grows.
fn build_docs() -> Vec<HashSet<u64>> {
    let mut rng = SplitMix64(SEED);
    let mut docs = Vec::with_capacity(N_DOCS);

    let mut first: HashSet<u64> = HashSet::new();
    while first.len() < CARDINALITY {
        first.insert(rng.range(UNIVERSE));
    }
    docs.push(first);

    for i in 1..N_DOCS {
        let prev = &docs[i - 1];
        let elements: Vec<u64> = prev.iter().copied().collect();
        let victim = elements[rng.range(elements.len() as u64) as usize];
        let mut replacement = rng.range(UNIVERSE);
        while prev.contains(&replacement) || replacement == victim {
            replacement = rng.range(UNIVERSE);
        }
        let mut next = prev.clone();
        next.remove(&victim);
        next.insert(replacement);
        docs.push(next);
    }
    docs
}

fn exact_jaccard(a: &HashSet<u64>, b: &HashSet<u64>) -> f64 {
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        1.0
    } else {
        inter as f64 / union as f64
    }
}

/// Ground truth ids for a query at threshold `tau`, sorted ascending.
fn truth_at(docs: &[HashSet<u64>], q_idx: usize, tau: f64) -> Vec<u32> {
    let q = &docs[q_idx];
    docs.iter()
        .enumerate()
        .filter(|(_, d)| exact_jaccard(q, d) >= tau)
        .map(|(i, _)| i as u32)
        .collect()
}

/// Ask the index for its top ranked candidates using
/// [`LshIndex::top_k`], then keep those scoring at or above `tau`.
///
/// `top_k` is the crate's built in refine helper: it runs the LSH
/// candidate step, scores each candidate through
/// [`MinHasher::estimate_jaccard_index`], and returns the ids sorted by
/// score. Dense signatures get the noisy register agreement estimator,
/// sparse signatures (both still sparse) get the exact set Jaccard.
/// Passing `k = N_DOCS` guarantees no candidate is dropped for
/// truncation reasons before the `tau` cutoff applies.
fn refined_via_top_k<K>(
    index: &LshIndex<K, P, BANDS, Store>,
    query: &K,
    tau: f64,
    state: &mut QueryState,
) -> Vec<u32>
where
    K: MinHasher<P>,
    K::Word: core::hash::Hash,
{
    let mut ids: Vec<u32> = index
        .top_k(query, N_DOCS, state)
        .into_iter()
        .filter(|&(_, score)| score >= tau)
        .map(|(id, _)| id)
        .collect();
    ids.sort_unstable();
    ids
}

/// Recall and precision, computed by a two pointer merge over the two
/// sorted id lists. Both lists are tiny (tens of ids), so this beats any
/// hashing scheme in both wall clock and allocations.
fn recall_precision(refined: &[u32], truth: &[u32]) -> (f64, f64) {
    let mut tp = 0usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < refined.len() && j < truth.len() {
        match refined[i].cmp(&truth[j]) {
            core::cmp::Ordering::Equal => {
                tp += 1;
                i += 1;
                j += 1;
            }
            core::cmp::Ordering::Less => i += 1,
            core::cmp::Ordering::Greater => j += 1,
        }
    }
    let recall = if truth.is_empty() {
        1.0
    } else {
        tp as f64 / truth.len() as f64
    };
    let precision = if refined.is_empty() {
        1.0
    } else {
        tp as f64 / refined.len() as f64
    };
    (recall, precision)
}

fn main() {
    let docs = build_docs();

    let dense_sigs: Vec<MinHash<u64, P>> =
        docs.iter().map(|d| d.iter().copied().collect()).collect();
    let sparse_sigs: Vec<SparseHashes<u64, P>> =
        docs.iter().map(|d| d.iter().copied().collect()).collect();

    let dense_index: LshIndex<MinHash<u64, P>, P, BANDS, Store> =
        LshIndex::from_signatures(dense_sigs.clone());
    let sparse_index: LshIndex<SparseHashes<u64, P>, P, BANDS, Store> =
        LshIndex::from_signatures(sparse_sigs.clone());

    let thresholds = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

    println!("threshold,backend,mean_recall,mean_precision,mean_returned_count");

    for &tau in &thresholds {
        let mut dense_recall_sum = 0.0;
        let mut dense_precision_sum = 0.0;
        let mut dense_returned_sum = 0.0;

        let mut sparse_recall_sum = 0.0;
        let mut sparse_precision_sum = 0.0;
        let mut sparse_returned_sum = 0.0;

        let mut state = QueryState::new();

        // Sample queries evenly across the chain so we cover every
        // neighbourhood, not just the head.
        let query_stride = N_DOCS / N_QUERIES;
        for i in 0..N_QUERIES {
            let q_idx = i * query_stride;
            let truth = truth_at(&docs, q_idx, tau);

            let refined_d = refined_via_top_k(&dense_index, &dense_sigs[q_idx], tau, &mut state);
            let (rec_d, prec_d) = recall_precision(&refined_d, &truth);
            dense_recall_sum += rec_d;
            dense_precision_sum += prec_d;
            dense_returned_sum += refined_d.len() as f64;

            let refined_s = refined_via_top_k(&sparse_index, &sparse_sigs[q_idx], tau, &mut state);
            let (rec_s, prec_s) = recall_precision(&refined_s, &truth);
            sparse_recall_sum += rec_s;
            sparse_precision_sum += prec_s;
            sparse_returned_sum += refined_s.len() as f64;
        }

        let n = N_QUERIES as f64;
        println!(
            "{tau:.2},dense,{:.4},{:.4},{:.2}",
            dense_recall_sum / n,
            dense_precision_sum / n,
            dense_returned_sum / n,
        );
        println!(
            "{tau:.2},sparse,{:.4},{:.4},{:.2}",
            sparse_recall_sum / n,
            sparse_precision_sum / n,
            sparse_returned_sum / n,
        );
    }
}
