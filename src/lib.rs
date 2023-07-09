#![feature(return_position_impl_trait_in_trait)]
#![doc = include_str!("../README.md")]

pub mod minhash;
pub mod zero;
pub mod maximal;
pub mod splitmix;
pub mod xorshift;
pub mod primitive;
pub mod min;
pub mod intersection;
pub mod from_iter;
pub mod minhash_array;
pub mod atomic;

pub mod prelude {
    pub use crate::minhash::MinHash;
    pub use crate::zero::Zero;
    pub use crate::min::Min;
    pub use crate::primitive::Primitive;
    pub use crate::maximal::Maximal;
    pub use crate::splitmix::SplitMix;
    pub use crate::xorshift::XorShift;
    pub use crate::intersection::*;
    pub use crate::minhash_array::*;
    pub use crate::atomic::*;
}