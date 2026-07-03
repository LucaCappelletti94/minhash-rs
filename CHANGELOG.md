# Changelog

All notable changes to this project are documented in this file. The format is
loosely based on Keep a Changelog, and the project follows semantic versioning.

## 0.5.0

### Breaking

- `MinHash<Word, PERMUTATIONS>` is now `MinHash<Word, PERMUTATIONS, H>` where `H` is a phantom [`Hasher`](crate::hasher::Hasher) type parameter defaulting to `SipHashes13`. Sketches built with different hashers are different Rust types, preventing silent correctness bugs from cross-hasher comparison.
- `insert_with_siphashes13()`, `insert_with_fnv()`, `insert_with_keyed_siphashes13()`, `insert_with_keyed_fnv()` are replaced by a single `insert()` method that dispatches through the hasher type.
- `may_contain_value_with_siphashes13()`, `may_contain_value_with_fnv()`, `may_contain_value_with_keyed_siphashes13()`, `may_contain_value_with_keyed_fnv()` are replaced by a single `may_contain()` method.
- Keyed hashers are constructed via `MinHash::<u64, 128, SipHashes13Keyed>::new_with_keys(k0, k1)` or `MinHash::<u64, 128, FnvKeyed>::new_with_keys(key, 0)` instead of passing keys to insert methods.
- `MinHashArray<Word, PERMUTATIONS, N>` is now `MinHashArray<Word, PERMUTATIONS, N, H>`.

### Added

- `hasher` module with `Hasher` trait and marker types: `SipHashes13`, `SipHashes13Keyed`, `Fnv`, `FnvKeyed`.
- `MinHash::new_with_keys()` and `MinHash::sparse_with_keys()` for keyed hasher construction.
- `MinHash::with_keys()` for setting keys after deserialization.
- `lsh` module providing locality-sensitive hashing banding for signatures. `MinHash::band_hashes::<BANDS>()` splits a signature into `BANDS` equal-sized bands.

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
