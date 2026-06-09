//! Tests for the union (merge) semantics of MinHash sketches.
//!
//! Combining two MinHash sketches with the bitwise-or operators takes the
//! element-wise minimum, which yields the sketch of the union of the two
//! underlying sets (not the intersection). These tests pin that behavior and
//! the algebraic properties a merge should have.
// `op_ref`: we intentionally exercise the `|` operator with a borrowed right
// operand. `float_cmp`: a merged sketch compared with itself is exactly 1.0.
#![allow(clippy::op_ref, clippy::float_cmp)]

use std::collections::HashSet;

use minhash_rs::prelude::*;

const PERMUTATIONS: usize = 256;

fn set(range: std::ops::RangeInclusive<u64>) -> HashSet<u64> {
    range.collect()
}

fn sketch(values: &HashSet<u64>) -> MinHash<u64, PERMUTATIONS> {
    values.iter().collect()
}

#[test]
fn bitor_equals_union_sketch() {
    let a = set(1..=8);
    let b = set(5..=12);
    let union: HashSet<u64> = a.union(&b).copied().collect();

    let merged = sketch(&a) | sketch(&b);
    assert_eq!(merged, sketch(&union));
}

#[test]
fn bitor_is_commutative() {
    let a = sketch(&set(1..=20));
    let b = sketch(&set(10..=40));
    assert_eq!(a | b, b | a);
}

#[test]
fn bitor_is_idempotent() {
    let a = sketch(&set(3..=30));
    assert_eq!(a | a, a);
}

#[test]
fn bitor_with_empty_is_identity() {
    let a = sketch(&set(1..=50));
    let empty = MinHash::<u64, PERMUTATIONS>::new();
    assert_eq!(a | empty, a);
    assert_eq!(empty | a, a);
}

#[test]
fn bitor_assign_owned_and_ref_agree() {
    let a = sketch(&set(1..=15));
    let b = sketch(&set(7..=25));

    let mut owned = a;
    owned |= b;

    let mut by_ref = a;
    by_ref |= &b;

    assert_eq!(owned, by_ref);
    assert_eq!(owned, a | b);
    assert_eq!(a | &b, a | b);
}

#[test]
fn iterator_union_equals_union_of_all_sets() {
    let a = set(1..=8);
    let b = set(5..=12);
    let c = set(20..=30);
    let all: HashSet<u64> = a.union(&b).copied().chain(c.iter().copied()).collect();

    let merged = vec![sketch(&a), sketch(&b), sketch(&c)].into_iter().union();

    assert_eq!(merged, sketch(&all));
}

#[test]
fn iterator_union_of_empty_iterator_is_empty() {
    let merged = std::iter::empty::<MinHash<u64, PERMUTATIONS>>().union();
    assert!(merged.is_empty());
}

#[test]
fn merged_sketch_estimates_self_jaccard_as_one() {
    // A merged (union) sketch compared against itself must have Jaccard 1.0.
    let merged = sketch(&set(1..=64)) | sketch(&set(40..=120));
    assert_eq!(merged.estimate_jaccard_index(&merged), 1.0);
}
