//! Property-based tests for the MinHash invariants, run across every word type.
//!
//! Example-based tests pin a handful of inputs; these assert structural laws
//! over arbitrary inputs that proptest shrinks to a minimal counterexample on
//! failure. The laws are exact (set-theoretic / algebraic), not statistical:
//! they hold for every input regardless of estimation error, so they make sharp
//! oracles rather than tolerance checks.

// Sketches compared with themselves yield exactly 1.0, so the strict float
// comparison is intentional.
#![allow(clippy::float_cmp)]

use minhash_rs::prelude::*;
use proptest::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

const PERMUTATIONS: usize = 64;

/// The full bound set the per-word helpers share.
trait Word:
    Min + XorShift + Copy + Ord + Eq + Maximal + core::fmt::Debug + Serialize + DeserializeOwned
where
    u64: Primitive<Self>,
{
}

impl<T> Word for T
where
    T: Min + XorShift + Copy + Ord + Eq + Maximal + core::fmt::Debug + Serialize + DeserializeOwned,
    u64: Primitive<T>,
{
}

fn build<W: Word>(values: &[u64]) -> MinHash<W, PERMUTATIONS>
where
    u64: Primitive<W>,
{
    let mut mh = MinHash::<W, PERMUTATIONS>::new();
    for &v in values {
        mh.insert_with_siphashes13(v);
    }
    mh
}

/// Every inserted value must be reported as possibly contained.
fn prop_no_false_negatives<W: Word>(values: &[u64])
where
    u64: Primitive<W>,
{
    let mh = build::<W>(values);
    for &v in values {
        assert!(
            mh.may_contain_value_with_siphashes13(v),
            "false negative for {v}"
        );
    }
}

/// A sketch depends only on the set of inserted values, not on their order or
/// multiplicity, since each word stores a minimum.
fn prop_insertion_order_invariant<W: Word>(values: &[u64])
where
    u64: Primitive<W>,
{
    let forward = build::<W>(values);
    let mut shuffled: Vec<u64> = values.iter().rev().copied().collect();
    shuffled.extend_from_slice(values); // duplicates must not matter either
    let reordered = build::<W>(&shuffled);
    assert_eq!(forward, reordered);
}

/// The serde representation round-trips back to an identical sketch.
fn prop_serde_roundtrip<W: Word>(values: &[u64])
where
    u64: Primitive<W>,
{
    let mh = build::<W>(values);
    let json = serde_json::to_string(&mh).expect("serialization failed");
    let decoded: MinHash<W, PERMUTATIONS> =
        serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(mh, decoded);
}

/// The estimated Jaccard index is a symmetric value in [0, 1], and a sketch is
/// always perfectly similar to itself.
fn prop_jaccard_bounds<W: Word>(a: &[u64], b: &[u64])
where
    u64: Primitive<W>,
{
    let sa = build::<W>(a);
    let sb = build::<W>(b);
    let j = sa.estimate_jaccard_index(&sb);
    assert!((0.0..=1.0).contains(&j), "jaccard {j} out of range");
    assert_eq!(j, sb.estimate_jaccard_index(&sa), "jaccard not symmetric");
    assert_eq!(sa.estimate_jaccard_index(&sa), 1.0, "self similarity != 1");
}

/// Union (bitwise-or) is commutative, associative, idempotent, has the empty
/// sketch as identity, and never introduces false negatives for either side.
fn prop_union_laws<W: Word>(a: &[u64], b: &[u64], c: &[u64])
where
    u64: Primitive<W>,
{
    let (sa, sb, sc) = (build::<W>(a), build::<W>(b), build::<W>(c));
    let empty = MinHash::<W, PERMUTATIONS>::new();

    assert_eq!(sa | sb, sb | sa, "union not commutative");
    assert_eq!((sa | sb) | sc, sa | (sb | sc), "union not associative");
    assert_eq!(sa | empty, sa, "empty sketch is not the union identity");
    // A second sketch over the same set is equal to the first, so their union
    // must reproduce it: union is idempotent.
    let sa_again = build::<W>(a);
    assert_eq!(sa | sa_again, sa, "union not idempotent");

    let union = sa | sb;
    for &v in a.iter().chain(b.iter()) {
        assert!(
            union.may_contain_value_with_siphashes13(v),
            "union dropped {v}"
        );
    }
}

/// Runs a single-set property against every supported word width.
macro_rules! for_each_word {
    ($prop:ident, $($arg:expr),+) => {{
        $prop::<u8>($($arg),+);
        $prop::<u16>($($arg),+);
        $prop::<u32>($($arg),+);
        $prop::<u64>($($arg),+);
        $prop::<usize>($($arg),+);
    }};
}

fn values() -> impl Strategy<Value = Vec<u64>> {
    prop::collection::vec(any::<u64>(), 0..40)
}

proptest! {
    #[test]
    fn no_false_negatives(v in values()) {
        for_each_word!(prop_no_false_negatives, &v);
    }

    #[test]
    fn insertion_order_invariant(v in values()) {
        for_each_word!(prop_insertion_order_invariant, &v);
    }

    #[test]
    fn serde_roundtrip(v in values()) {
        for_each_word!(prop_serde_roundtrip, &v);
    }

    #[test]
    fn jaccard_bounds(a in values(), b in values()) {
        for_each_word!(prop_jaccard_bounds, &a, &b);
    }

    #[test]
    fn union_laws(a in values(), b in values(), c in values()) {
        for_each_word!(prop_union_laws, &a, &b, &c);
    }
}
