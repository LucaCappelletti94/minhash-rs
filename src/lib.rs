#![doc = include_str!("../README.md")]

pub mod atomic;
pub mod from_iter;
pub mod intersection;
pub mod maximal;
pub mod min;
pub mod minhash;
pub mod minhash_array;
pub mod primitive;
pub mod splitmix;
pub mod xorshift;
pub mod zero;

pub mod prelude {
    pub use crate::atomic::*;
    pub use crate::intersection::*;
    pub use crate::maximal::Maximal;
    pub use crate::min::Min;
    pub use crate::minhash::MinHash;
    pub use crate::minhash_array::*;
    pub use crate::primitive::Primitive;
    pub use crate::splitmix::SplitMix;
    pub use crate::xorshift::XorShift;
    pub use crate::zero::Zero;
}
