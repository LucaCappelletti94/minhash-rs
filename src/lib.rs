#![doc = include_str!("../README.md")]
#![no_std]

pub mod atomic;
pub mod from_iter;
pub mod maximal;
pub mod min;
pub mod minhash;
pub mod minhash_array;
pub mod primitive;
pub mod splitmix;
pub mod union;
pub mod xorshift;

/// Re-exports of the traits and types needed to use the crate.
pub mod prelude {
    pub use crate::atomic::*;
    pub use crate::maximal::Maximal;
    pub use crate::min::Min;
    pub use crate::minhash::MinHash;
    pub use crate::minhash_array::*;
    pub use crate::primitive::Primitive;
    pub use crate::splitmix::SplitMix;
    pub use crate::union::*;
    pub use crate::xorshift::XorShift;
}
