#![doc = include_str!("../README.md")]
#![no_std]

pub mod atomic;
pub mod from_iter;
pub mod hasher;
pub mod hashtype;
pub mod lsh;
pub mod maximal;
pub mod min_hasher;
pub mod minhash;
pub mod minhash_array;
pub mod primitive;
pub mod sparse_hashes;
pub mod sparse_values;
pub mod union;
pub mod xorshift;

/// Re-exports of the traits and types needed to use the crate.
pub mod prelude {
    pub use crate::atomic::*;
    pub use crate::hasher::*;
    pub use crate::hashtype::{HashType, SparseArithmetic};
    pub use crate::lsh::*;
    pub use crate::maximal::Maximal;
    pub use crate::min_hasher::{MinHasher, Outcome};
    pub use crate::minhash::MinHash;
    pub use crate::minhash_array::*;
    pub use crate::primitive::{Primitive, SparseFor};
    pub use crate::sparse_hashes::SparseHashes;
    pub use crate::sparse_values::SparseValues;
    pub use crate::union::*;
    pub use crate::xorshift::XorShift;
}
