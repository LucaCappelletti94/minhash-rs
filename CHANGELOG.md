# Changelog

All notable changes to this project are documented in this file. The format is
loosely based on Keep a Changelog, and the project follows semantic versioning.

## 0.5.0

### Breaking

- `MinHash<Word, PERMUTATIONS>` is now `MinHash<Word, PERMUTATIONS, H, Hash>` where `H` is a phantom [`Hasher`](crate::hasher::Hasher) type parameter defaulting to `SipHashes13` and `Hash` is a phantom [`HashType`](crate::hashtype::HashType) parameter for the internal hash stream width, defaulting to `u64`. Sketches built with different hashers or different hash widths are different Rust types, preventing silent correctness bugs from cross-configuration comparison.
- `insert_with_siphashes13()`, `insert_with_fnv()`, `may_contain_value_with_siphashes13()`, and `may_contain_value_with_fnv()` are replaced by single `insert()` and `may_contain()` methods that dispatch through the hasher and hash-type parameters.
- Keyed hashers (`SipHashes13Keyed`, `FnvKeyed`, `MinHash::new_with_keys`, `MinHash::sparse_with_keys`, `MinHash::with_keys`) are removed, along with the runtime `keys` field on the sketch. Callers who need domain separation or a per-instance key can hash a `(key, value)` tuple instead.
- The `SplitMix`, `ToU64`, and `SparseWord` traits are removed. Their jobs are now covered by [`HashType::splitmix`](crate::hashtype::HashType::splitmix), [`Primitive`](crate::primitive::Primitive) in the reverse direction (`Word: Primitive<Hash>`), and the [`SparseFor<Hash>`](crate::primitive::SparseFor) marker that gates the sparse constructor.
- Sparse mode is available whenever `Word: SparseFor<Hash>` (that is, when the word can round-trip a full-width digest). New pair supported: `MinHash<u32, PERMUTATIONS, H, u32>` gets sparse mode at half the digest storage.
- The dense hash stream now iterates on the `Hash` type rather than the `Word` type, so signatures produced by 0.5.0 will not compare equal to 0.4.x sketches for the same input set. Estimation accuracy is unaffected.
- `MinHashArray<Word, PERMUTATIONS, N>` is now `MinHashArray<Word, PERMUTATIONS, N, H, Hash>`.
- The atomic insertion API drops keyed variants. `AtomicFetchInsert` exposes `fetch_insert_with_siphashes13::<V, Hash>` and `fetch_insert_with_fnv::<V, Hash>`, with an explicit `Hash` type parameter so the caller opts into the hash width matching the sketch.

### Added

- `hashtype` module with the [`HashType`] trait (`splitmix` + `XorShift` + `SparseArithmetic` bundle) and impls for `u64` and `u32`. The `u32` variant enables a half-memory sparse mode. The `u32` `splitmix` uses Chris Wellons's public-domain "lowbias32" mixer.
- `SparseArithmetic` trait bundling `wrapping_add`, `wrapping_sub`, `saturating_add`, and `saturating_sub` for the sparse encoding pipeline. It supersedes the earlier `WrappingArithmetic` name.
- `hasher` module with `Hasher` trait and marker types: `SipHashes13`, `Fnv`.
- `lsh` module providing locality-sensitive hashing banding for signatures. `MinHash::band_hashes::<BANDS>()` splits a signature into `BANDS` equal-sized bands.

### Fixed

- Sparse mode used to encode digests as `digest.wrapping_add(1)`, so a digest equal to `Hash::MAX` wrapped to zero, colliding with the sparse mode flag and silently dropping the insert plus every later value in sort order. The encoding is now `saturating_add`, which folds `MAX` and `MAX - 1` into the same slot (a benign 1-in-2^N false positive) without ever producing a zero encoding. The old scheme's collision rate was 1 in 2^32 per insert for `Hash = u32`, high enough to be reached by a moderately busy application. `Hash = u64` was in practice unreachable at 1 in 2^64.
- The `sparse_union` scratch buffer no longer uses `MaybeUninit::assume_init` on a generic `[Word; PERMUTATIONS]`, which was unsound for any external `Word` impl whose valid bit patterns are restricted. The buffer is now initialised with `Word::maximal()` at the same performance and without the unsafe block.
- The `sparse()` constructor's `PERMUTATIONS >= 2` guarantee is now enforced at compile time. Previously the `const ASSERT_PERMUTATIONS` in the sparse impl block was decorated `#[allow(dead_code)]` and never referenced, so it never fired: `MinHash::<_, 1>::sparse()` silently constructed a broken sketch that densified on the first insert. `sparse()` now consumes the const with `let () = Self::ASSERT_PERMUTATIONS;`, turning misuse into a clean `E0080` at the call site.
- The dense constructor `new()` now carries a compile-time `PERMUTATIONS >= 1` assertion. Previously a zero-permutation sketch constructed cleanly and then panicked with a generic bounds-check on the first `is_sparse`, `insert`, `may_contain`, `is_empty`, or `is_full`, since all of them touch `words[0]`. The failure is now caught at monomorphisation.
- `dense_jaccard` no longer divides by zero for a zero-permutation sketch. With the new `new()` const assertion in place this branch is unreachable in normal use, but the guard makes the helper self-contained.
- `MinHashArray`'s `Debug` and `Clone` are now hand-rolled instead of derived. The derive auto-generated `H: Debug + Clone` and `Hash: Debug + Clone` bounds even though both fields are `PhantomData`, so a user-defined `Hasher` marker that did not itself derive those traits broke printing and cloning downstream. The hand-rolled impls only bind `Word` and now also expose `Copy` when `Word: Copy`, so a `MinHashArray<u64, P, N>` can be moved by value like a plain array.
- `MinHash::band_hashes::<BANDS>()` now rejects `BANDS = 0`, `BANDS > PERMUTATIONS`, and any `BANDS` that does not evenly divide `PERMUTATIONS` at compile time via an `AssertBandsDivide` generic helper struct's associated const. Previously all three cases silently produced meaningless output: `BANDS = 0` returned `[u64; 0]`, `BANDS > PERMUTATIONS` collapsed every band to the FNV of the empty slice (destroying LSH candidate generation), and non-dividing `BANDS` dropped the tail registers. Misuse is now a hard `E0080` at the call site.

### Changed (cleanup)

- Dropped the redundant `Word: XorShift` bound from every `where` clause in `src/atomic.rs`, `src/minhash.rs`, and `src/minhash_array.rs`, and from the four downstream test files that carried it. The XorShift stream lives entirely on the `Hash` type (`u64` or `u32`), so the bound was dead weight forcing users of custom `Word` types to implement a trait method that was never called.
- Deleted the `impl XorShift for u8`, `for u16`, and `for usize` — dead code that only satisfied the redundant bound above. The `XorShift` trait itself stays public because `HashType: XorShift` legitimately needs it.
- Dropped `Maximal` from `HashType`'s supertrait list. `Hash::maximal()` was never called anywhere in the crate. The new supertrait bundle is `HashType: XorShift + SparseArithmetic + Copy + Eq`, matching what the trait's methods actually use.

## 0.4.0


## 0.3.0

This release contains several breaking changes alongside correctness fixes,
hardening and tooling. Upgrading from 0.2 requires the migration notes below.

### Breaking

- Renamed the misspelled `fvn` hash methods to `fnv` (Fowler-Noll-Vo), for
  example `insert_with_fnv` and `may_contain_value_with_fnv`.
- Reworked the atomic insertion API. The unsound `transmute` of a shared
  `&[Word]` into a slice of atomics was replaced by `AsAtomic::as_atomic`, which
  derives the atomic view from an exclusive `&mut self` borrow. Concurrent
  inserts now go through `minhash.as_atomic().fetch_insert_with_*(...)` instead
  of calling the insert methods directly on the sketch.
- Renamed the set-combining operation from `intersection` to `union` and changed
  the operator from `&` (`BitAnd`) to `|` (`BitOr`). The operation always
  produced the union (merge) sketch, never the intersection, so the name and
  operator were corrected.
- Removed the unused `Zero` trait and the unreachable `Min`/`Maximal`
  implementations for `u128`.
- Trimmed the `Primitive` trait to the `u64`-source conversions actually used by
  the crate.
- `XorShift::xorshift` now takes `self` by value instead of `&mut self`.

### Fixed

- Fixed undefined behavior in the atomic insertion path (see above), now
  verified under Miri.
- Fixed small word types collapsing to a saturated sketch from a single
  insertion: the hash generator no longer emits zero (which is an XorShift fixed
  point), so a value whose seed truncated to zero no longer wipes the sketch.
- Implemented `Maximal` for `usize`, which was missing and prevented constructing
  a `MinHash` over `usize`.
- `is_full` now tests against the smallest reachable hash value (one) rather than
  zero, which is no longer reachable.

### Added

- Serde `Serialize`/`Deserialize` for `MinHash` and `MinHashArray` (using
  `serde-big-array` for the const-generic arrays).
- The crate is now `#![no_std]`. The atomic implementations are gated on
  `target_has_atomic`, so the core sketch works on targets without the relevant
  atomics.
- Documentation for the full public API, a usage example, a strict lint gate
  (clippy pedantic and cargo, plus `missing_docs`), code coverage in CI, and a
  Miri job covering the atomic path across every word width.

### Notes

- Minimum supported Rust version is 1.75.
