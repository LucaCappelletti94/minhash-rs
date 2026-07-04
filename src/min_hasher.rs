//! [`MinHasher`] trait and [`Outcome`] enum.
//!
//! Implemented by every sketch in this crate. Parameterised on
//! `P` (number of permutations) and `Value` (input type, defaults to `u64`).
//! `MinHash` and `SparseHashes` implement it blanket over any `V: CoreHash`.
//! `SparseValues` implements it only for `Value = u64` because its codec
//! buffer stores raw `u64` inputs.

use core::hash::Hash as CoreHash;

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::minhash::{dense_jaccard, MinHash};
use crate::primitive::Primitive;

/// Result of a single [`MinHasher::insert`] call.
///
/// Dense sketches only ever return [`Outcome::Inserted`]. Sparse wrappers
/// return [`Outcome::Duplicate`] on repeated inputs and [`Outcome::Promoted`]
/// on the specific insert that pays the `O(P^2)` sparse-to-dense promotion
/// tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The value was inserted.
    Inserted,
    /// The value was already present. No state changed.
    Duplicate,
    /// The insert triggered a sparse-to-dense promotion. Every subsequent
    /// insert on this sketch is `O(P)` again, matching a from-scratch dense
    /// sketch.
    Promoted,
}

/// Trait implemented by every MinHash-family sketch in this crate.
///
/// `P` is the number of permutations. `Value` is the input type each impl
/// accepts. Two sketches whose `Value`, `Word`, `Hash`, and `Hasher` agree
/// can be Jaccard-compared through the trait without an adapter.
pub trait MinHasher<const P: usize, Value: CoreHash = u64>: Sized {
    /// Storage word type for the dense signature.
    type Word: Ord + Copy + Maximal + Primitive<Self::Hash>;
    /// Internal hash-stream width.
    type Hash: HashType + Primitive<Self::Word>;
    /// Phantom hasher marker (e.g. `SipHashes13`, `Fnv`).
    type Hasher: Hasher;

    /// Insert a value into the sketch. See [`Outcome`] for the return
    /// contract.
    fn insert(&mut self, value: Value) -> Outcome;

    /// Return `true` if the sketch may contain `value`. Dense sketches
    /// inherit the classical MinHash false-positive profile. Sparse wrappers
    /// are exact while still sparse.
    fn may_contain(&self, value: Value) -> bool;

    /// Force the sketch into its dense representation in place. No-op on
    /// sketches already dense.
    fn densify(&mut self);

    /// Materialise a dense [`MinHash`] equivalent to the current state.
    fn to_dense(&self) -> MinHash<Self::Word, P, Self::Hasher, Self::Hash>;

    /// Estimate the Jaccard similarity between two sketches.
    ///
    /// The right-hand side is any sketch sharing the same `Word`, `Hash`,
    /// `Hasher`, and `Value`, so cross-wrapper comparisons work through the
    /// default implementation. Both operands are materialised as dense
    /// sketches and compared via the classical register-agreement fraction.
    fn estimate_jaccard_index<Rhs>(&self, other: &Rhs) -> f64
    where
        Rhs: MinHasher<P, Value, Word = Self::Word, Hash = Self::Hash, Hasher = Self::Hasher>,
    {
        let a = self.to_dense();
        let b = other.to_dense();
        dense_jaccard::<Self::Word, P>(a.as_words(), b.as_words())
    }
}
