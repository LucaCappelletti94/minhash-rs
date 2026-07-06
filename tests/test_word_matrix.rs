//! Coverage-focused tests that walk a (Word, Hash) matrix to hit every
//! monomorphisation of MinHash and SparseHashes methods.
//!
//! Jaccard estimates against an identical sketch are exactly 1.0, so the
//! strict float comparisons here are intentional.
#![allow(clippy::float_cmp)]

use minhash_rs::prelude::*;

const P: usize = 64;

// ── MinHash: new, words, from_words, into_words, is_empty ───────────────

#[test]
fn minhash_new_and_words_across_word_matrix() {
    // (u32, u32)
    {
        let mh = MinHash::<u32, P, SipHashes13, u32>::new();
        assert!(mh.is_empty());
        assert_eq!(mh.as_words().len(), P);
        let words: [u32; P] = *mh.as_words();
        let rebuilt = MinHash::<u32, P, SipHashes13, u32>::from_words(words);
        assert!(rebuilt.is_empty());
        let consumed = mh.into_words();
        assert_eq!(consumed.len(), P);
        let _mut_ref: &mut [u32; P] = MinHash::<u32, P, SipHashes13, u32>::new().as_words_mut();
    }

    // (u32, u64)
    {
        let mh = MinHash::<u32, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        assert_eq!(mh.as_words().len(), P);
        let words: [u32; P] = *mh.as_words();
        let rebuilt = MinHash::<u32, P, SipHashes13, u64>::from_words(words);
        assert!(rebuilt.is_empty());
        let consumed = mh.into_words();
        assert_eq!(consumed.len(), P);
        let _mut_ref: &mut [u32; P] = MinHash::<u32, P, SipHashes13, u64>::new().as_words_mut();
    }

    // (u64, u32)
    {
        let mh = MinHash::<u64, P, SipHashes13, u32>::new();
        assert!(mh.is_empty());
        assert_eq!(mh.as_words().len(), P);
        let words: [u64; P] = *mh.as_words();
        let rebuilt = MinHash::<u64, P, SipHashes13, u32>::from_words(words);
        assert!(rebuilt.is_empty());
        let consumed = mh.into_words();
        assert_eq!(consumed.len(), P);
        let _mut_ref: &mut [u64; P] = MinHash::<u64, P, SipHashes13, u32>::new().as_words_mut();
    }

    // (u64, u64)
    {
        let mh = MinHash::<u64, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        assert_eq!(mh.as_words().len(), P);
        let words: [u64; P] = *mh.as_words();
        let rebuilt = MinHash::<u64, P, SipHashes13, u64>::from_words(words);
        assert!(rebuilt.is_empty());
        let consumed = mh.into_words();
        assert_eq!(consumed.len(), P);
        let _mut_ref: &mut [u64; P] = MinHash::<u64, P, SipHashes13, u64>::new().as_words_mut();
    }

    // (usize, u64)
    {
        let mh = MinHash::<usize, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        assert_eq!(mh.as_words().len(), P);
        let words: [usize; P] = *mh.as_words();
        let rebuilt = MinHash::<usize, P, SipHashes13, u64>::from_words(words);
        assert!(rebuilt.is_empty());
        let consumed = mh.into_words();
        assert_eq!(consumed.len(), P);
        let _mut_ref: &mut [usize; P] = MinHash::<usize, P, SipHashes13, u64>::new().as_words_mut();
    }
}

// ── MinHash: insert, may_contain, is_empty transition ─────────────────────

#[test]
fn minhash_insert_may_contain_across_word_matrix() {
    let values: [u64; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    // (u32, u32, SipHashes13)
    {
        let mut mh = MinHash::<u32, P, SipHashes13, u32>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(
                mh.may_contain(v),
                "u32/u32 SipHashes13 false negative for {v}"
            );
        }
    }

    // (u32, u64, SipHashes13)
    {
        let mut mh = MinHash::<u32, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(
                mh.may_contain(v),
                "u32/u64 SipHashes13 false negative for {v}"
            );
        }
    }

    // (u64, u32, SipHashes13)
    {
        let mut mh = MinHash::<u64, P, SipHashes13, u32>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(
                mh.may_contain(v),
                "u64/u32 SipHashes13 false negative for {v}"
            );
        }
    }

    // (u64, u64, SipHashes13)
    {
        let mut mh = MinHash::<u64, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(
                mh.may_contain(v),
                "u64/u64 SipHashes13 false negative for {v}"
            );
        }
    }

    // (usize, u64, SipHashes13)
    {
        let mut mh = MinHash::<usize, P, SipHashes13, u64>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(
                mh.may_contain(v),
                "usize/u64 SipHashes13 false negative for {v}"
            );
        }
    }

    // (u32, u32, Fnv)
    {
        let mut mh = MinHash::<u32, P, Fnv, u32>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(mh.may_contain(v), "u32/u32 Fnv false negative for {v}");
        }
    }

    // (u64, u64, Fnv)
    {
        let mut mh = MinHash::<u64, P, Fnv, u64>::new();
        assert!(mh.is_empty());
        for &v in &values {
            mh.insert(v);
        }
        assert!(!mh.is_empty());
        for &v in &values {
            assert!(mh.may_contain(v), "u64/u64 Fnv false negative for {v}");
        }
    }
}

// ── MinHash: estimate_jaccard_index ───────────────────────────────────────

#[test]
fn minhash_estimate_jaccard_matches_across_matrix() {
    // (u32, u32)
    {
        let mut a = MinHash::<u32, P, SipHashes13, u32>::new();
        let mut b = MinHash::<u32, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u32/u32 Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);
    }

    // (u32, u64)
    {
        let mut a = MinHash::<u32, P, SipHashes13, u64>::new();
        let mut b = MinHash::<u32, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u32/u64 Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);
    }

    // (u64, u32)
    {
        let mut a = MinHash::<u64, P, SipHashes13, u32>::new();
        let mut b = MinHash::<u64, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u64/u32 Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);
    }

    // (u64, u64)
    {
        let mut a = MinHash::<u64, P, SipHashes13, u64>::new();
        let mut b = MinHash::<u64, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u64/u64 Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);
    }

    // (usize, u64)
    {
        let mut a = MinHash::<usize, P, SipHashes13, u64>::new();
        let mut b = MinHash::<usize, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "usize/u64 Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);
    }
}

// ── MinHash: union via |= ────────────────────────────────────────────────

#[test]
fn minhash_union_across_matrix() {
    // (u32, u32)
    {
        let mut a = MinHash::<u32, P, SipHashes13, u32>::new();
        let mut b = MinHash::<u32, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let mut merged = a;
        merged |= &b;
        for v in 0..100u64 {
            assert!(merged.may_contain(v), "u32/u32 union missing {v}");
        }
    }

    // (u32, u64)
    {
        let mut a = MinHash::<u32, P, SipHashes13, u64>::new();
        let mut b = MinHash::<u32, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let mut merged = a;
        merged |= &b;
        for v in 0..100u64 {
            assert!(merged.may_contain(v), "u32/u64 union missing {v}");
        }
    }

    // (u64, u32)
    {
        let mut a = MinHash::<u64, P, SipHashes13, u32>::new();
        let mut b = MinHash::<u64, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let mut merged = a;
        merged |= &b;
        for v in 0..100u64 {
            assert!(merged.may_contain(v), "u64/u32 union missing {v}");
        }
    }

    // (u64, u64)
    {
        let mut a = MinHash::<u64, P, SipHashes13, u64>::new();
        let mut b = MinHash::<u64, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let mut merged = a;
        merged |= &b;
        for v in 0..100u64 {
            assert!(merged.may_contain(v), "u64/u64 union missing {v}");
        }
    }

    // (usize, u64)
    {
        let mut a = MinHash::<usize, P, SipHashes13, u64>::new();
        let mut b = MinHash::<usize, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let mut merged = a;
        merged |= &b;
        for v in 0..100u64 {
            assert!(merged.may_contain(v), "usize/u64 union missing {v}");
        }
    }
}

// ── MinHash: is_full ──────────────────────────────────────────────────────

#[test]
fn minhash_is_full_across_matrix() {
    // u32 word: fresh sketch is not full, becomes full after inserts.
    {
        let mh = MinHash::<u32, P, SipHashes13, u32>::new();
        assert!(!mh.is_full());
        let mut mh = mh;
        for v in 0..2000u64 {
            mh.insert(v);
        }
        assert!(mh.is_full(), "u32/u32 should be full after 2000 inserts");
    }

    // u64 word: fresh sketch is not full, one insert fills all slots
    // because the hash stream produces 64 non-maximal u64 values.
    {
        let mh = MinHash::<u64, P, SipHashes13, u64>::new();
        assert!(!mh.is_full());
        let mut mh = mh;
        mh.insert(42u64);
        assert!(mh.is_full(), "u64/u64 should be full after one insert");
    }
}

// ── SparseHashes: full lifecycle across the sparse matrix ────────────────

#[test]
#[allow(clippy::clone_on_copy)]
fn sparse_hashes_matrix_full_lifecycle() {
    // (u64, u64)
    {
        let sh = SparseHashes::<u64, P, SipHashes13, u64>::new();
        assert!(sh.is_sparse());
        assert!(!sh.is_dense());
        assert_eq!(sh.count(), 0);
        let _ = format!("{sh:?}");
        let _ = SparseHashes::<u64, P, SipHashes13, u64>::default();
        let cloned = sh.clone();
        assert!(cloned.is_sparse());
    }

    // (u32, u32)
    {
        let sh = SparseHashes::<u32, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        assert!(!sh.is_dense());
        assert_eq!(sh.count(), 0);
        let _ = format!("{sh:?}");
        let _ = SparseHashes::<u32, P, SipHashes13, u32>::default();
        let cloned = sh.clone();
        assert!(cloned.is_sparse());
    }

    // (u64, u32)
    {
        let sh = SparseHashes::<u64, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        assert!(!sh.is_dense());
        assert_eq!(sh.count(), 0);
        let _ = format!("{sh:?}");
        let _ = SparseHashes::<u64, P, SipHashes13, u32>::default();
        let cloned = sh.clone();
        assert!(cloned.is_sparse());
    }

    // (usize, u32)
    {
        let sh = SparseHashes::<usize, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        assert!(!sh.is_dense());
        assert_eq!(sh.count(), 0);
        let _ = format!("{sh:?}");
        let _ = SparseHashes::<usize, P, SipHashes13, u32>::default();
        let cloned = sh.clone();
        assert!(cloned.is_sparse());
    }

    // (usize, u64)
    {
        let sh = SparseHashes::<usize, P, SipHashes13, u64>::new();
        assert!(sh.is_sparse());
        assert!(!sh.is_dense());
        assert_eq!(sh.count(), 0);
        let _ = format!("{sh:?}");
        let _ = SparseHashes::<usize, P, SipHashes13, u64>::default();
        let cloned = sh.clone();
        assert!(cloned.is_sparse());
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn sparse_hashes_insert_and_promotion_across_matrix() {
    // (u64, u64) -- insert, may_contain, promote, From
    {
        let mut sh = SparseHashes::<u64, P, SipHashes13, u64>::new();
        assert!(sh.is_sparse());
        let before_count = sh.count();
        assert_eq!(before_count, 0);

        // Insert a few values, stay sparse.
        for v in 0..5u64 {
            let outcome = sh.insert(v);
            assert!(
                matches!(outcome, Outcome::Inserted),
                "expected Inserted, got {outcome:?}"
            );
        }
        assert!(sh.is_sparse());
        assert!(sh.count() > 0);

        // may_contain on stored values.
        for v in 0..5u64 {
            assert!(
                sh.may_contain(v),
                "u64/u64 sparse may_contain false negative for {v}"
            );
        }
        assert!(
            !sh.may_contain(999u64),
            "u64/u64 sparse should not contain 999"
        );

        // Drive to promotion.
        let mut promoted = false;
        for v in 5..(P as u64 + 10) {
            let outcome = sh.insert(v);
            if matches!(outcome, Outcome::Promoted) {
                promoted = true;
                break;
            }
        }
        assert!(promoted, "u64/u64 should have promoted");
        assert!(!sh.is_sparse());
        assert!(sh.is_dense());

        // may_contain still works in dense mode.
        assert!(sh.may_contain(0u64));

        // MinHash::from
        let mh: MinHash<u64, P, SipHashes13, u64> = MinHash::from(sh);
        assert!(mh.may_contain(0u64));

        // From::from
        let mut sh2 = SparseHashes::<u64, P, SipHashes13, u64>::new();
        sh2.insert(42u64);
        let mh2: MinHash<u64, P, SipHashes13, u64> = MinHash::from(sh2);
        assert!(mh2.may_contain(42u64));
    }

    // (u32, u32)
    {
        let mut sh = SparseHashes::<u32, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        for v in 0..5u64 {
            sh.insert(v);
        }
        for v in 0..5u64 {
            assert!(
                sh.may_contain(v),
                "u32/u32 sparse may_contain false negative for {v}"
            );
        }

        let mut promoted = false;
        for v in 5..(P as u64 + 10) {
            let outcome = sh.insert(v);
            if matches!(outcome, Outcome::Promoted) {
                promoted = true;
                break;
            }
        }
        assert!(promoted, "u32/u32 should have promoted");
        assert!(sh.is_dense());

        let mh: MinHash<u32, P, SipHashes13, u32> = MinHash::from(sh);
        assert!(mh.may_contain(0u64));

        let mut sh2 = SparseHashes::<u32, P, SipHashes13, u32>::new();
        sh2.insert(42u64);
        let mh2: MinHash<u32, P, SipHashes13, u32> = MinHash::from(sh2);
        assert!(mh2.may_contain(42u64));
    }

    // (u64, u32)
    {
        let mut sh = SparseHashes::<u64, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        for v in 0..5u64 {
            sh.insert(v);
        }
        for v in 0..5u64 {
            assert!(
                sh.may_contain(v),
                "u64/u32 sparse may_contain false negative for {v}"
            );
        }

        let mut promoted = false;
        for v in 5..(P as u64 + 10) {
            let outcome = sh.insert(v);
            if matches!(outcome, Outcome::Promoted) {
                promoted = true;
                break;
            }
        }
        assert!(promoted, "u64/u32 should have promoted");

        let mh: MinHash<u64, P, SipHashes13, u32> = MinHash::from(sh);
        assert!(mh.may_contain(0u64));
    }

    // (usize, u32)
    {
        let mut sh = SparseHashes::<usize, P, SipHashes13, u32>::new();
        assert!(sh.is_sparse());
        for v in 0..5u64 {
            sh.insert(v);
        }
        for v in 0..5u64 {
            assert!(
                sh.may_contain(v),
                "usize/u32 sparse may_contain false negative for {v}"
            );
        }

        let mut promoted = false;
        for v in 5..(P as u64 + 10) {
            let outcome = sh.insert(v);
            if matches!(outcome, Outcome::Promoted) {
                promoted = true;
                break;
            }
        }
        assert!(promoted, "usize/u32 should have promoted");

        let mh: MinHash<usize, P, SipHashes13, u32> = MinHash::from(sh);
        assert!(mh.may_contain(0u64));
    }

    // (usize, u64)
    {
        let mut sh = SparseHashes::<usize, P, SipHashes13, u64>::new();
        assert!(sh.is_sparse());
        for v in 0..5u64 {
            sh.insert(v);
        }
        for v in 0..5u64 {
            assert!(
                sh.may_contain(v),
                "usize/u64 sparse may_contain false negative for {v}"
            );
        }

        let mut promoted = false;
        for v in 5..(P as u64 + 10) {
            let outcome = sh.insert(v);
            if matches!(outcome, Outcome::Promoted) {
                promoted = true;
                break;
            }
        }
        assert!(promoted, "usize/u64 should have promoted");

        let mh: MinHash<usize, P, SipHashes13, u64> = MinHash::from(sh);
        assert!(mh.may_contain(0u64));
    }
}

#[test]
fn sparse_hashes_jaccard_and_densify_across_matrix() {
    // (u64, u64) -- sparse-sparse Jaccard, mixed-mode Jaccard, densify
    {
        let mut a = SparseHashes::<u64, P, SipHashes13, u64>::new();
        let mut b = SparseHashes::<u64, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        // Both sparse.
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u64/u64 sparse-sparse Jaccard out of bounds: {j}"
        );
        assert_eq!(a.estimate_jaccard_index(&a), 1.0);

        // Promote a, compare mixed.
        for v in 5..(P as u64 + 10) {
            a.insert(v);
        }
        assert!(a.is_dense());
        assert!(b.is_sparse());
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u64/u64 mixed Jaccard out of bounds: {j}"
        );

        // densify on already-dense is no-op.
        a.densify();
        assert!(a.is_dense());
    }

    // (u32, u32)
    {
        let mut a = SparseHashes::<u32, P, SipHashes13, u32>::new();
        let mut b = SparseHashes::<u32, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u32/u32 Jaccard out of bounds: {j}"
        );

        for v in 5..(P as u64 + 10) {
            a.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u32/u32 mixed Jaccard out of bounds: {j}"
        );
    }

    // (u64, u32)
    {
        let mut a = SparseHashes::<u64, P, SipHashes13, u32>::new();
        let mut b = SparseHashes::<u64, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "u64/u32 Jaccard out of bounds: {j}"
        );
    }

    // (usize, u32)
    {
        let mut a = SparseHashes::<usize, P, SipHashes13, u32>::new();
        let mut b = SparseHashes::<usize, P, SipHashes13, u32>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "usize/u32 Jaccard out of bounds: {j}"
        );
    }

    // (usize, u64)
    {
        let mut a = SparseHashes::<usize, P, SipHashes13, u64>::new();
        let mut b = SparseHashes::<usize, P, SipHashes13, u64>::new();
        for v in 0..50u64 {
            a.insert(v);
        }
        for v in 50..100u64 {
            b.insert(v);
        }
        let j = a.estimate_jaccard_index(&b);
        assert!(
            (0.0..=1.0).contains(&j),
            "usize/u64 Jaccard out of bounds: {j}"
        );
    }
}

#[test]
fn sparse_hashes_minhasher_trait_via_fully_qualified() {
    // Call the MinHasher trait methods on SparseHashes<u64, 64, SipHashes13, u64>
    // using fully qualified syntax to ensure the trait impl entry points are
    // exercised.
    let mut sh = SparseHashes::<u64, P, SipHashes13, u64>::new();

    // <SparseHashes as MinHasher>::insert
    let outcome = <SparseHashes<u64, P, SipHashes13, u64> as MinHasher<P>>::insert(&mut sh, 42u64);
    assert!(matches!(outcome, Outcome::Inserted));

    // <SparseHashes as MinHasher>::may_contain
    assert!(<SparseHashes<u64, P, SipHashes13, u64> as MinHasher<P>>::may_contain(&sh, 42u64));

    // <SparseHashes as MinHasher>::densify converts sparse digests to dense.
    <SparseHashes<u64, P, SipHashes13, u64> as MinHasher<P>>::densify(&mut sh);
    assert!(sh.is_dense());

    // MinHash::from converts a densified SparseHashes to MinHash
    let dense: MinHash<u64, P, SipHashes13, u64> = MinHash::from(sh);
    assert!(dense.may_contain(42u64));
}
