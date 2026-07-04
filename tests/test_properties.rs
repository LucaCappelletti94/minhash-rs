//! Property-based tests for the MinHash invariants, run across every
//! `(Word, Hash)` combination the crate supports.
//!
//! Example-based tests pin a handful of inputs. These assert structural laws
//! over arbitrary inputs that proptest shrinks to a minimal counterexample on
//! failure. The laws are exact (set-theoretic and algebraic), not
//! statistical, so they make sharp oracles rather than tolerance checks.
//!
//! Two matrices are exercised. `for_each_word_hash!` walks the full 5 x 2
//! Cartesian product of dense-supported `(Word, Hash)` pairs (five word
//! widths crossed with `Hash = u64` and `Hash = u32`).
//! `for_each_sparse_pair!` walks the subset for which `Word: SparseFor<Hash>`
//! holds, exercising the sparse round-trip specifically.

// Sketches compared with themselves yield exactly 1.0, so the strict float
// comparison is intentional.
#![allow(clippy::float_cmp)]

use minhash_rs::prelude::*;
use minhash_rs::primitive::Primitive;
use proptest::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

const PERMUTATIONS: usize = 64;

/// The full bound set that dense property helpers share, parameterised by the
/// hash stream width.
trait Word<H>:
    Copy + Ord + Maximal + Primitive<H> + core::fmt::Debug + Serialize + DeserializeOwned
where
    H: HashType + Primitive<Self>,
{
}

impl<T, H> Word<H> for T
where
    T: Copy + Ord + Maximal + Primitive<H> + core::fmt::Debug + Serialize + DeserializeOwned,
    H: HashType + Primitive<T>,
{
}

fn build<W, H>(values: &[u64]) -> MinHash<W, PERMUTATIONS, SipHashes13, H>
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let mut mh = MinHash::<W, PERMUTATIONS, SipHashes13, H>::new();
    for &v in values {
        mh.insert(v);
    }
    mh
}

fn build_sparse<W, H>(values: &[u64]) -> MinHash<W, PERMUTATIONS, SipHashes13, H>
where
    W: Word<H> + SparseFor<H>,
    H: HashType + Primitive<W>,
{
    let mut mh = MinHash::<W, PERMUTATIONS, SipHashes13, H>::sparse();
    for &v in values {
        mh.insert(v);
    }
    mh
}

/// Every inserted value must be reported as possibly contained.
fn prop_no_false_negatives<W, H>(values: &[u64])
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let mh = build::<W, H>(values);
    for &v in values {
        assert!(mh.may_contain(v), "false negative for {v}");
    }
}

/// A sketch depends only on the set of inserted values, not on their order or
/// multiplicity, since each register stores a minimum.
fn prop_insertion_order_invariant<W, H>(values: &[u64])
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let forward = build::<W, H>(values);
    let mut shuffled: Vec<u64> = values.iter().rev().copied().collect();
    shuffled.extend_from_slice(values);
    let reordered = build::<W, H>(&shuffled);
    assert_eq!(forward, reordered);
}

/// The serde representation round-trips back to an identical sketch.
fn prop_serde_roundtrip<W, H>(values: &[u64])
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let mh = build::<W, H>(values);
    let json = serde_json::to_string(&mh).expect("serialization failed");
    let decoded: MinHash<W, PERMUTATIONS, SipHashes13, H> =
        serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(mh, decoded);
}

/// The estimated Jaccard index is a symmetric value in [0, 1], and a sketch
/// is always perfectly similar to itself.
fn prop_jaccard_bounds<W, H>(a: &[u64], b: &[u64])
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let sa = build::<W, H>(a);
    let sb = build::<W, H>(b);
    let j = sa.estimate_jaccard_index(&sb);
    assert!((0.0..=1.0).contains(&j), "jaccard {j} out of range");
    assert_eq!(j, sb.estimate_jaccard_index(&sa), "jaccard not symmetric");
    assert_eq!(sa.estimate_jaccard_index(&sa), 1.0, "self similarity != 1");
}

/// Union (bitwise-or) is commutative, associative, idempotent, has the empty
/// sketch as identity, and never introduces false negatives for either side.
fn prop_union_laws<W, H>(a: &[u64], b: &[u64], c: &[u64])
where
    W: Word<H>,
    H: HashType + Primitive<W>,
{
    let (sa, sb, sc) = (build::<W, H>(a), build::<W, H>(b), build::<W, H>(c));
    let empty = MinHash::<W, PERMUTATIONS, SipHashes13, H>::new();

    assert_eq!(sa | sb, sb | sa, "union not commutative");
    assert_eq!((sa | sb) | sc, sa | (sb | sc), "union not associative");
    assert_eq!(sa | empty, sa, "empty sketch is not the union identity");
    let sa_again = build::<W, H>(a);
    assert_eq!(sa | sa_again, sa, "union not idempotent");

    let union = sa | sb;
    for &v in a.iter().chain(b.iter()) {
        assert!(union.may_contain(v), "union dropped {v}");
    }
}

/// Sparse mode obeys the same no-false-negative invariant as dense mode. This
/// exercises the sparse encoding (`saturating_add`) and the sparse membership
/// lookup end-to-end for every valid `(Word, Hash)` sparse pair.
fn prop_sparse_no_false_negatives<W, H>(values: &[u64])
where
    W: Word<H> + SparseFor<H>,
    H: HashType + Primitive<W>,
{
    let mh = build_sparse::<W, H>(values);
    for &v in values {
        assert!(mh.may_contain(v), "sparse false negative for {v}");
    }
}

/// The dense sketch of a set equals the densified sparse sketch of the same
/// set. Exercises the sparse round-trip (`saturating_add` on encode,
/// `wrapping_sub` on decode) and `densify_into` across the sparse matrix.
fn prop_sparse_dense_equivalence<W, H>(values: &[u64])
where
    W: Word<H> + SparseFor<H>,
    H: HashType + Primitive<W>,
{
    let dense = build::<W, H>(values);
    let sparse = build_sparse::<W, H>(values);
    // `PartialEq` densifies the sparse operand internally, so this compares
    // the two on identical dense signatures.
    assert_eq!(dense, sparse);
    assert_eq!(sparse, dense);
}

/// Cross-hasher, cross-`Hash`-width sketches are different types, so any
/// attempt to compare them or union them is a compile error, not a runtime
/// silent bug. This test just documents the guarantee.
#[allow(dead_code)]
fn _cross_config_is_a_compile_error() {
    // The following four lines would be compile errors and are therefore
    // commented out:
    //
    //   let a: MinHash<u64, 64, SipHashes13, u64> = MinHash::new();
    //   let b: MinHash<u64, 64, Fnv, u64> = MinHash::new();
    //   let _ = a == b;                       // hashers differ
    //   let _ = a | b;                        // hashers differ
    //   let c: MinHash<u64, 64, SipHashes13, u32> = MinHash::new();
    //   let _ = a == c;                       // hash widths differ
}

// ─── Combinatorial helpers ──────────────────────────────────────────────────

/// Runs a single-set property across the full 5 x 2 dense matrix
/// (5 word widths x {u64, u32} hash widths).
macro_rules! for_each_word_hash {
    ($prop:ident, $($arg:expr),+) => {{
        $prop::<u8,    u64>($($arg),+);
        $prop::<u16,   u64>($($arg),+);
        $prop::<u32,   u64>($($arg),+);
        $prop::<u64,   u64>($($arg),+);
        $prop::<usize, u64>($($arg),+);
        $prop::<u8,    u32>($($arg),+);
        $prop::<u16,   u32>($($arg),+);
        $prop::<u32,   u32>($($arg),+);
        $prop::<u64,   u32>($($arg),+);
        $prop::<usize, u32>($($arg),+);
    }};
}

/// Runs a single-set property across every `(Word, Hash)` pair for which
/// sparse mode is available (i.e. `Word: SparseFor<Hash>`). The
/// `usize` + `u64` pair is 64-bit only.
macro_rules! for_each_sparse_pair {
    ($prop:ident, $($arg:expr),+) => {{
        $prop::<u64,   u64>($($arg),+);
        $prop::<u32,   u32>($($arg),+);
        $prop::<u64,   u32>($($arg),+);
        $prop::<usize, u32>($($arg),+);
        #[cfg(target_pointer_width = "64")]
        $prop::<usize, u64>($($arg),+);
    }};
}

fn values() -> impl Strategy<Value = Vec<u64>> {
    prop::collection::vec(any::<u64>(), 0..40)
}

proptest! {
    #[test]
    fn no_false_negatives(v in values()) {
        for_each_word_hash!(prop_no_false_negatives, &v);
    }

    #[test]
    fn insertion_order_invariant(v in values()) {
        for_each_word_hash!(prop_insertion_order_invariant, &v);
    }

    #[test]
    fn serde_roundtrip(v in values()) {
        for_each_word_hash!(prop_serde_roundtrip, &v);
    }

    #[test]
    fn jaccard_bounds(a in values(), b in values()) {
        for_each_word_hash!(prop_jaccard_bounds, &a, &b);
    }

    #[test]
    fn union_laws(a in values(), b in values(), c in values()) {
        for_each_word_hash!(prop_union_laws, &a, &b, &c);
    }

    #[test]
    fn sparse_no_false_negatives(v in values()) {
        for_each_sparse_pair!(prop_sparse_no_false_negatives, &v);
    }

    #[test]
    fn sparse_dense_equivalence(v in values()) {
        for_each_sparse_pair!(prop_sparse_dense_equivalence, &v);
    }
}
