# Changelog

All notable changes to this project are documented in this file. The format is
loosely based on Keep a Changelog, and the project follows semantic versioning.

## 0.6.0
### Breaking

- `MinHasher` trait's `Value` is now an associated type instead of a type parameter. `MinHash` and `SparseHashes` gain a `Value` generic parameter defaulting to `u64`. Migration for external consumers. `MinHash<u64, 128>` keeps working thanks to the default. `impl MinHasher<128, u64>` bounds become `impl MinHasher<128>` (or `impl MinHasher<128, Value = u64>` to constrain `Value` at the bound). Inherent `estimate_jaccard_index` / `insert` / `may_contain` / `densify` / `band_hashes` methods on the three sketch types are gone in favour of the trait methods. The trait's `to_dense` method and the sparse wrappers' inherent `into_minhash` are removed, replaced by the standard `From<SparseHashes<...>> for MinHash<...>` and `From<SparseValues<...>> for MinHash<...>` conversions (`MinHash::from(sketch)` or `sketch.into()`).

- `MinHash` is now strictly the dense Broder `k`-mins signature. The sparse-mode dispatch, the `sparse()` constructor, and every `is_sparse` / `densify` helper are removed. Sparse-mode logic moves to the two new wrapper types.
- Two new wrappers ship: `SparseHashes<Word, PERMUTATIONS, H, Hash>` (bottom-k list of hash digests, direct replacement for the old `MinHash::sparse()`) and `SparseValues<PERMUTATIONS, H, Hash, Code>` (raw `u64` values under a `dsi-bitstream` code, replaces the `MinHashValues` type from 0.5). Both wrap an inner `MinHash` and promote in place at capacity.
- New sealed `MinHasher<const P: usize>` trait unifies the three sketch types with `type Value` as an associated type. `MinHash` and `SparseHashes` implement it once per concrete `(Word, PERMUTATIONS, H, Hash, Value)` tuple with `type Value = Value` set from the struct's fifth type parameter. `SparseValues` fixes `Value = u64` at the trait impl because its codec buffer stores raw `u64` inputs. Trait methods are `insert`, `may_contain`, `densify`, `estimate_jaccard_index(&self, other: &Self)`, and `band_hashes::<const BANDS>()`, each impl provides its own body. Cross-variant Jaccard is done explicitly by converting both operands to `MinHash` with `MinHash::from(sketch)` or `sketch.into()` and comparing the resulting dense sketches. The sparse-sparse exact fast path fires automatically through the trait for same-variant sparse operands. External types cannot implement this trait.
- `insert` on the sparse wrappers returns `Outcome { Inserted, Duplicate, Promoted }` so callers can observe the `O(P^2)` promotion event. `MinHash`'s identity impl always returns `Inserted`.
- `MinHash`'s `PartialEq` / `Eq` / `CoreHash` impls simplify to plain per-slot comparison. New `MinHash::from_words`, `as_words`, `as_words_mut`, and `into_words` accessors expose the raw slot array for the wrappers to seed and inspect.
- `From<SparseHashes<...>> for MinHash<...>` and `From<SparseValues<...>> for MinHash<...>` for the natural `wrapper.into()` conversion.
- `MinHashValues` renamed to `SparseValues`.
- Retired `benches/bench_sparse.rs`. Wrapper-specific benches will return in a follow-up.

### Added

- `sketching-core` and `dsi-bitstream = "0.9"` as direct dependencies for the codec surface. `sketching_core::sparse_value_list` ships the codec markers (`ConstCode<CODE>`, `SigBitsCode`, and every `dsi-bitstream` code accessible via `ConstCode`).
- `impl FromIterator<Value> for SparseHashes<Word, PERMUTATIONS, H, Hash>` blanket over `V: CoreHash`, and `impl FromIterator<u64> for SparseValues<PERMUTATIONS, H, Hash, Code>`. `iter.collect::<SparseHashes<u64, 128>>()` and `iter.collect::<SparseValues<128>>()` now work alongside the existing `MinHash` counterpart.
- Sparse-mode inspection methods on both wrappers: `SparseHashes::distinct_hashes(&self) -> Option<usize>`, `SparseHashes::hashes(&self) -> Option<impl Iterator<Item = Hash>>` (ascending), `SparseValues::distinct_values(&self) -> Option<usize>`, and `SparseValues::values(&self) -> Option<ValueIter<'_, BE, Code>>` (descending, zero-allocation). All return `None` once the sketch has densified, so callers cannot accidentally observe the empty tail buffer as an empty set.
- `pub mod index` behind the new default `alloc` feature: a compile-time banded LSH index `LshIndex<K, const PERMUTATIONS, const BANDS, S>` over any `MinHasher<PERMUTATIONS>`. Type-level `Store` and `NoStore` markers gate whether the index keeps signatures for a Jaccard refine pass or drops them after banding. Query is one binary search per band followed by a collision-count aggregation, backed by a caller-owned `QueryState` scratch for zero-allocation queries. Ranked `candidates` and `top_k` (Store only) build on the same primitive. Turning `alloc` off yields the previous no-`Vec` surface unchanged.
- `LshIndex` band tables use a compressed sparse row layout. A single flat `Vec<BandEntry>` plus a `[u32; BANDS]` of per-band start offsets replaces the previous `[Vec<(u64, u32)>; BANDS]`, so build allocates once instead of once per band and the query hot path walks one contiguous vector.
- `LshIndex::from_signatures_par` behind the new `rayon` feature. Byte-identical output to the sequential `from_signatures`. Roughly `4.8x` speedup at 1M signatures on a Threadripper 5975WX with `P = 128, BANDS = 16`. Sequential remains the default.
- Optional `epserde` feature exposes `LshIndexRepr<E, BANDS>`, a persistent representation of the LSH band tables. `LshIndex::into_repr` and `LshIndex::from_repr` handle the owned round-trip on the `NoStore` variant. Save with `unsafe { repr.store(path) }`. Load with `unsafe { LshIndexRepr::<Vec<BandEntry>, BANDS>::mmap(path, Flags::empty()) }` for a `LshIndexRepr<&[BandEntry], BANDS>` whose entries slice points directly into the mapped file, or `load_full` for an owned copy. `LshIndexRepr::candidates` runs on both forms so the same query code drives in-memory and mmap-loaded indices.
- Batched `FromIterator` on `MinHash`, `SparseHashes`, and `SparseValues` folds every input in one pass through the fixed-size batching buffer, reaching roughly `3x` throughput over the per-element `insert` loop on cold sketches.
- `examples/bench_sparse_lsh_recall.rs` measures LSH refine recall and precision for dense vs sparse backends on chain-generated documents, emitting the raw numbers as CSV to stdout. `figures/sparse_lsh_recall.png` in the repo captures the plot.


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
