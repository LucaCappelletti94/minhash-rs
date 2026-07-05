//! Batched build primitives for the `FromIterator` path on [`MinHash`]
//! and its sparse wrappers.
//!
//! The streaming [`insert`](crate::minhash::MinHash::insert) path folds
//! a length-`PERMUTATIONS` xorshift chain per input element, with each
//! per-slot step depending on the previous slot's state. That makes the
//! per-element cost strictly serial in the number of permutations.
//!
//! This module inverts the loop order. Given a bounded scratch buffer
//! of `Hash` values, it fills the buffer via `hash + splitmix^2` in one
//! pass and then, for each permutation slot, advances every digest one
//! xorshift step and reduces the minimum into that slot. The per-slot
//! reduction has no cross-iteration dependency on the accumulator
//! except the min, so the loop-carried min chain gets broken into four
//! parallel accumulators and the effective throughput is a few cycles
//! per (input, slot) pair instead of the ~10 cycle chain step of the
//! streaming path.
//!
//! The scratch buffer lives on the stack, so this path works under
//! `no_std` without the `alloc` feature. Input iterators of any length
//! are processed by refilling the same buffer in chunks.
//!
//! # Zero guards
//!
//! `MinHash`'s streaming `HashStream` applies two zero
//! guards on every step: one on the raw `Hash` after each xorshift and
//! one on the narrowed `Word`. The `Hash`-level guard is redundant
//! given a nonzero seed from phase 1, because the crate's xorshift
//! constants are chosen so that xorshift is a bijection on the nonzero
//! space of both `u64` and `u32`. The batched path drops it.
//!
//! The `Word`-level guard is needed only when narrowing from `Hash` to
//! `Word` can turn a nonzero `Hash` into a zero `Word`. That is exactly
//! what [`Primitive::IS_LOSSLESS`] tracks. When it is `true`, both
//! guards are provably no-ops and phase 2 skips them; when it is
//! `false`, phase 2 keeps the word guard so the sparse-mode flag
//! `words[0] == 0` invariant is preserved.

use core::hash::{Hash as CoreHash, Hasher as _};

use crate::hasher::Hasher;
use crate::hashtype::HashType;
use crate::maximal::Maximal;
use crate::primitive::Primitive;

/// Stack-only scratch chunk size in `Hash` elements. 512 `u64` values
/// is 4 KiB, comfortably in L1 across cores, and gives phase 1 enough
/// amortization to be a small fraction of the total.
const CHUNK: usize = 512;

/// Hash a value under `H` and narrow the resulting `u64` digest to
/// `Hash`. Shared with `MinHash::hash_value`
/// but taken as a free function so the batched path does not have to
/// name a `MinHash` type parameter.
#[inline]
fn hash_value<H, Hash, V>(value: V) -> Hash
where
    H: Hasher,
    Hash: HashType,
    V: CoreHash,
{
    let mut hasher = H::build();
    value.hash(&mut hasher);
    Hash::from_u64_digest(hasher.finish())
}

/// Phase 2: for each slot of `words`, advance every digest one xorshift
/// step, narrow to `Word`, and reduce the minimum into that slot. Four
/// parallel accumulators break the loop-carried min dependency; the
/// tail loop merges into `a` only because `b`, `c`, `d` stay at the
/// initial `seed` and the final `min` fold reconciles that. The word
/// guard is only emitted when the narrow is lossy.
#[inline]
#[allow(clippy::many_single_char_names)]
fn phase2_reduce<Word, const P: usize, Hash>(words: &mut [Word; P], digests: &mut [Hash])
where
    Word: Ord + Copy + Maximal + PartialEq,
    Hash: HashType + Primitive<Word>,
{
    let zero_word: Word = Hash::ZERO.convert();
    let one_word: Word = Hash::ONE.convert();

    for word in words.iter_mut() {
        let seed = *word;
        let mut a = seed;
        let mut b = seed;
        let mut c = seed;
        let mut d = seed;

        let mut chunks = digests.chunks_exact_mut(4);
        for chunk in chunks.by_ref() {
            let x0 = chunk[0].xorshift();
            let x1 = chunk[1].xorshift();
            let x2 = chunk[2].xorshift();
            let x3 = chunk[3].xorshift();
            chunk[0] = x0;
            chunk[1] = x1;
            chunk[2] = x2;
            chunk[3] = x3;
            let w0: Word = maybe_guard::<Word, Hash>(x0.convert(), zero_word, one_word);
            let w1: Word = maybe_guard::<Word, Hash>(x1.convert(), zero_word, one_word);
            let w2: Word = maybe_guard::<Word, Hash>(x2.convert(), zero_word, one_word);
            let w3: Word = maybe_guard::<Word, Hash>(x3.convert(), zero_word, one_word);
            a = w0.min(a);
            b = w1.min(b);
            c = w2.min(c);
            d = w3.min(d);
        }
        for tail in chunks.into_remainder() {
            let x = tail.xorshift();
            *tail = x;
            let w: Word = maybe_guard::<Word, Hash>(x.convert(), zero_word, one_word);
            a = w.min(a);
        }
        *word = a.min(b).min(c).min(d);
    }
}

/// Word-level zero guard. Compiles to a no-op when the narrow from
/// `Hash` to `Word` is lossless, because then the `if` branch reduces
/// to `if false` at monomorphization.
#[inline]
fn maybe_guard<Word, Hash>(w: Word, zero_word: Word, one_word: Word) -> Word
where
    Word: Copy + PartialEq,
    Hash: Primitive<Word>,
{
    if <Hash as Primitive<Word>>::IS_LOSSLESS {
        w
    } else if w == zero_word {
        one_word
    } else {
        w
    }
}

/// Drive phase 1 + [`phase2_reduce`] over a `Hash`-typed stack
/// buffer of `CHUNK` slots, pulling from an iterator until it runs dry.
///
/// The state living in `words` is treated as the current per-slot
/// minima; the reduction only lowers it. Callers that want a fresh
/// build pass `words` initialised to [`Maximal::maximal`]; callers that
/// want to extend an already-populated dense signature (for example,
/// the sparse wrappers after a densify) pass their current words in.
#[inline]
pub(crate) fn build_into<Word, const P: usize, H, Hash, V, I>(words: &mut [Word; P], iter: I)
where
    Word: Ord + Copy + Maximal + PartialEq,
    H: Hasher,
    Hash: HashType + Primitive<Word>,
    V: CoreHash,
    I: IntoIterator<Item = V>,
{
    let mut buf = [Hash::ZERO; CHUNK];
    let mut n = 0usize;
    for v in iter {
        let mut s = hash_value::<H, Hash, V>(v).splitmix().splitmix();
        if s == Hash::ZERO {
            s = Hash::ONE;
        }
        buf[n] = s;
        n += 1;
        if n == CHUNK {
            phase2_reduce::<Word, P, Hash>(words, &mut buf);
            n = 0;
        }
    }
    phase2_reduce::<Word, P, Hash>(words, &mut buf[..n]);
}
