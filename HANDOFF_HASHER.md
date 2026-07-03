# Handoff: Hasher as Phantom Type Generic

## Goal

Make the hash function a compile-time phantom type parameter of `MinHash` so that sketches built with different hashers are distinct types. This prevents silent correctness bugs from comparing Jaccard estimates or equality across incompatible hash streams.

## Current State

`MinHash<Word, PERMUTATIONS>` carries no hasher identity. The hasher is chosen at insert time via method suffixes:

- `insert_with_siphashes13()` / `may_contain_value_with_siphashes13()`
- `insert_with_keyed_siphashes13(key0, key1)` / `may_contain_value_with_keyed_siphashes13()`
- `insert_with_fnv()` / `may_contain_value_with_fnv()`
- `insert_with_keyed_fnv(key)` / `may_contain_value_with_keyed_fnv()`

`FromIterator` hardcodes SipHasher13. Atomic inserts mirror the same four variants.

Two sketches with identical memory layout but different hashers are the same Rust type. Comparing them produces garbage Jaccard estimates silently.

## Target State

`MinHash<Word, PERMUTATIONS, Hasher>` where `Hasher` is a phantom type. Sketches from different hashers are different types; cross-hasher comparison is a compile error.

## Design

### Hasher Trait

```rust
/// A hash strategy that can produce a stream of `Word`-sized hashes from a value.
pub trait Hasher {
    /// The concrete hasher type (e.g., `SipHasher13`, `FnvHasher`).
    type Concrete: core::hash::Hasher;

    /// Build a new hasher instance.
    fn build() -> Self::Concrete;

    /// For keyed hashers: build with explicit keys.
    /// Returns `None` if the hasher does not support custom keys.
    fn build_with_keys(_keys: &[u64]) -> Option<Self::Concrete> { None }
}
```

### Hasher Markers

```rust
/// SipHash-1-3 with default keys.
pub struct SipHashes13;
impl Hasher for SipHashes13 {
    type Concrete = siphasher::sip128::SipHasher13;
    fn build() -> Self::Concrete { SipHasher13::new() }
}

/// SipHash-1-3 with custom keys (2 keys passed at construction).
pub struct SipHashes13Keyed(pub u64, pub u64);
impl Hasher for SipHashes13Keyed {
    type Concrete = siphasher::sip128::SipHasher13;
    fn build() -> Self::Concrete { SipHasher13::new_with_keys(self.0, self.1) }
    fn build_with_keys(keys: &[u64]) -> Option<Self::Concrete> {
        if keys.len() >= 2 { Some(SipHasher13::new_with_keys(keys[0], keys[1])) } else { None }
    }
}

/// FNV-1a with default key.
pub struct Fnv;
impl Hasher for Fnv {
    type Concrete = fnv::FnvHasher;
    fn build() -> Self::Concrete { FnvHasher::default() }
}

/// FNV-1a with custom key.
pub struct FnvKeyed(pub u64);
impl Hasher for FnvKeyed {
    type Concrete = fnv::FnvHasher;
    fn build() -> Self::Concrete { FnvHasher::with_key(self.0) }
    fn build_with_keys(keys: &[u64]) -> Option<Self::Concrete> {
        keys.first().copied().map(FnvHasher::with_key)
    }
}
```

### MinHash Type

```rust
#[repr(transparent)]
pub struct MinHash<Word, const PERMUTATIONS: usize, Hasher = SipHashes13> {
    words: [Word; PERMUTATIONS],
    _hasher: core::marker::PhantomData<Hasher>,
}
```

Default type parameter `Hasher = SipHashes13` keeps `MinHash::<u64, 128>::new()` working as before (SipHash-1-3).

### Insert API

Method suffixes drop. Single `insert()` dispatches through the hasher:

```rust
impl<Word, const P: usize, H: Hasher> MinHash<Word, P, H> {
    pub fn insert<V: core::hash::Hash>(&mut self, value: V) {
        let mut hasher = H::build();
        value.hash(&mut hasher);
        let digest = hasher.finish();
        // ... sparse or dense insert as before
    }
}
```

Usage:

```rust
// SipHash-1-3 (default):
let mut a = MinHash::<u64, 128>::new();
a.insert(42);

// Explicit SipHash-1-3:
let mut a = MinHash::<u64, 128, SipHashes13>::new();
a.insert(42);

// Keyed SipHash:
let mut a = MinHash::<u64, 128, SipHashes13Keyed>::new_with_keys(k0, k1);
a.insert(42);

// FNV:
let mut a = MinHash::<u64, 128, Fnv>::new();
a.insert(42);
```

### May Contain API

Single `may_contain()` method:

```rust
pub fn may_contain<V: core::hash::Hash>(&self, value: V) -> bool {
    let mut hasher = H::build();
    value.hash(&mut hasher);
    let digest = hasher.finish();
    // ... sparse or dense check
}
```

### Atomic Inserts

The atomic slice stays as `&[AtomicU64]` (no hasher-aware wrapper). The `AtomicFetchInsert` trait methods drop suffixes:

```rust
pub trait AtomicFetchInsert {
    type Word;
    fn fetch_insert<H: Hash>(&self, value: H, ordering: Ordering);
    fn fetch_insert_with_keys<H: Hash>(&self, value: H, key0: u64, key1: u64, ordering: Ordering);
}
```

The caller is responsible for ensuring the hasher used at `fetch_insert` matches the hasher the parent `MinHash` was constructed with. Mismatched hashers produce incorrect sketches. This is documented prominently on `as_atomic()` and `fetch_insert`.

### FromIterator

```rust
impl<Word, A, H, const P: usize> FromIterator<A> for MinHash<Word, P, H>
where
    Word: Ord + Maximal + XorShift + ToU64,
    A: core::hash::Hash,
    H: Hasher,
    u64: Primitive<Word>,
{
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        let mut mh = Self::new();
        for item in iter {
            mh.insert(item);
        }
        mh
    }
}
```

### Jaccard and Equality

These already require `Self` receivers. With the hasher phantom, cross-hasher calls become type errors automatically:

```rust
let a: MinHash<u64, 128, SipHashes13> = (0..100).collect();
let b: MinHash<u64, 128, Fnv> = (0..100).collect();
a.estimate_jaccard_index(&b); // compile error: type mismatch
```

### Union

Same story: `BitOr` and `BitOrAssign` require matching types. Cross-hasher union is a compile error.

### Sparse Mode

The `sparse()` constructor and sparse mode logic are hasher-agnostic. The hasher phantom does not affect sparse/dense dispatch. Sparse digests are `u64` regardless of hasher.

## Migration Path

### Step 1: Add Hasher trait and markers

New module `src/hasher.rs`. Zero breaking changes yet.

### Step 2: Add phantom type parameter to MinHash

Add `Hasher = SipHashes13` as default. Update all impl blocks. `#[repr(transparent)]` is preserved (PhantomData is zero-sized).

### Step 3: Add `insert()` and `may_contain()` methods

Keep old suffixed methods as `#[deprecated]` aliases during a transition period. Or remove immediately for a clean break (0.4.0 -> 0.5.0).

### Step 4: Update FromIterator, union, atomic

Update trait impls to carry the hasher type. Update `AsAtomic` to return a hasher-aware atomic view.

### Step 5: Update tests and benchmarks

Every test that constructs a MinHash gains a type parameter. Most can rely on the default. Benchmarks that use FNV need explicit type annotations.

### Step 6: Remove deprecated suffixed methods

After migration, remove `insert_with_*` and `may_contain_value_with_*`.

## Breaking Changes

- `MinHash<Word, PERMUTATIONS>` -> `MinHash<Word, PERMUTATIONS, Hasher>`
- `insert_with_siphashes13()` -> `insert()`
- `may_contain_value_with_siphashes13()` -> `may_contain()`
- `FromIterator` now requires the hasher type to be known at the collection site
- Cross-hasher comparison/union/Jaccard become compile errors (previously silent bugs)
- `AsAtomic::as_atomic()` returns `&[AtomicU64]` (unchanged), documented that hasher must match

## Risks and Mitigations

**Risk**: Default type parameter means `MinHash::<u64, 128>::new()` still compiles but is SipHash-only. Users migrating code that mixed hashers will hit type errors at comparison sites, not construction sites.

**Mitigation**: Clear error messages via a custom trait bound or a compile-time error macro if cross-hasher use is detected.

**Risk**: Serde serialization. The hasher is a phantom type -- it does not affect the serialized bytes. Deserialization requires the hasher type to be known at the deserialization site.

**Mitigation**: This is acceptable. The caller must specify `serde_json::from_str::<MinHash<u64, 128, SipHashes13>>(&json)`. The bytes are identical across hashers.


## Files to Modify

| File | Change |
|------|--------|
| `src/lib.rs` | Add `hasher` module to prelude |
| `src/hasher.rs` | New: Hasher trait and marker types |
| `src/minhash.rs` | Add phantom type param, replace suffixed methods with `insert()`/`may_contain()` |
| `src/from_iter.rs` | Add hasher generic to FromIterator impl |
| `src/union.rs` | Add hasher generic to BitOr/BitOrAssign impls |
| `src/atomic.rs` | Document hasher matching requirement on `as_atomic()` and `fetch_insert` |
| `src/minhash_array.rs` | Add hasher generic to MinHashArray |
| `src/lsh.rs` | Add hasher generic to band_hashes |
| `tests/*.rs` | Update type annotations |
| `benches/*.rs` | Update type annotations |
| `README.md` | Update examples |
| `CHANGELOG.md` | Document breaking changes |

## Estimated Effort

Medium. The core change (phantom type parameter + replacing suffixed methods) is straightforward. The atomic API redesign is the most complex piece. Tests and benchmarks need mechanical updates. Total: 1-2 days of focused work.
