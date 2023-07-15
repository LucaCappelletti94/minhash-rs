use std::collections::HashSet;
/// This test module tests the effectiveness of MinHash to estimate the Jaccard index
/// of several large sets, starting from smaller one of 100 elements and upwards to sets
/// of 100_000_000 elements. The sets are generated randomly using the splitmix and xorshift
/// methods that are made available from the libraryr. We experimentally compare the effectiveness
/// of the MinHash for different word types (u8, u16, u32 and u64) and different number of permutations, and write the results
/// to a CSV file including also the time and memory (in terms of number of bits) required to
/// calculate the MinHash.
///
use std::fs::File;
use std::io::Write;

use hyperloglog_rs::prelude::*;
use indicatif::ParallelProgressIterator;
use minhash_rs::prelude::*;
use rayon::prelude::*;

/// Return set with up to the provided number of elements.
fn populate_set(elements: usize, mut random_state: u64) -> HashSet<u64> {
    random_state = random_state.splitmix();

    (0..elements)
        .map(|_| {
            random_state = random_state.xorshift();
            random_state % elements as u64
        })
        .collect()
}

/// Method to compute and write the results for a given constant parametrization of MinHash
/// to a CSV file so to avoid code duplication as much as possible.
fn estimate_jaccard_index_minhash_for_permutation<
    const PERMUTATIONS: usize,
    Word: Maximal + Copy + Min + Eq + XorShift,
>(
    elements: usize,
    first_set: &HashSet<u64>,
    second_set: &HashSet<u64>,
    real_jaccard: f64,
    mut file: &File,
) -> Result<(), std::io::Error>
where
    u64: minhash_rs::primitive::Primitive<Word>,
{
    let start = std::time::Instant::now();

    let minhash1: MinHash<Word, PERMUTATIONS> = first_set.iter().collect();
    let minhash2: MinHash<Word, PERMUTATIONS> = second_set.iter().collect();

    let estimated_jaccard = minhash1.estimate_jaccard_index(&minhash2);
    let end = std::time::Instant::now();

    file.write_all(
        format!(
            "{},{},{},{},{},{},{}\n",
            elements,
            PERMUTATIONS,
            core::any::type_name::<Word>(),
            minhash1.memory(),
            estimated_jaccard,
            real_jaccard,
            end.duration_since(start).as_micros()
        )
        .as_bytes(),
    )
}

/// Method to compute and write the results for a given constant parametrization of HLL
/// to a CSV file so to avoid code duplication as much as possible.
fn estimate_jaccard_index_hll_for_bits<PRECISION: Precision + WordType<BITS>, const BITS: usize>(
    elements: usize,
    first_set: &HashSet<u64>,
    second_set: &HashSet<u64>,
    real_jaccard: f64,
    mut file: &File,
) -> Result<(), std::io::Error> {
    let start = std::time::Instant::now();

    let hll1: HyperLogLog<PRECISION, BITS> = first_set.iter().collect();
    let hll2: HyperLogLog<PRECISION, BITS> = second_set.iter().collect();

    let estimated_jaccard = hll1.estimate_jaccard_cardinality(&hll2);

    let end = std::time::Instant::now();

    file.write_all(
        format!(
            "{},{},{},{},{},{},{}\n",
            elements,
            PRECISION::EXPONENT,
            BITS,
            (hll1.get_number_of_registers() + hll1.get_number_of_padding_registers()) * BITS,
            estimated_jaccard,
            real_jaccard,
            end.duration_since(start).as_micros()
        )
        .as_bytes(),
    )
}

/// Method to compute and write the results for a given constant parametrization of HLL
/// to a CSV file so to avoid code duplication as much as possible.
fn estimate_jaccard_index_hll<
    PRECISION: Precision + WordType<1> + WordType<2> + WordType<3> + WordType<4> + WordType<5> + WordType<6>,
>(
    elements: usize,
    first_set: &HashSet<u64>,
    second_set: &HashSet<u64>,
    real_jaccard: f64,
    file: &File,
) -> Result<(), std::io::Error> {
    estimate_jaccard_index_hll_for_bits::<PRECISION, 1>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_hll_for_bits::<PRECISION, 2>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_hll_for_bits::<PRECISION, 3>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_hll_for_bits::<PRECISION, 4>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_hll_for_bits::<PRECISION, 5>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_hll_for_bits::<PRECISION, 6>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;

    Ok(())
}

/// Method to compute and write the results for a given constant parametrization of MinHash
/// to a CSV file so to avoid code duplication as much as possible.
fn estimate_jaccard_index_minhash<Word: Maximal + Copy + Min + Eq + XorShift>(
    elements: usize,
    first_set: &HashSet<u64>,
    second_set: &HashSet<u64>,
    real_jaccard: f64,
    file: &File,
) -> Result<(), std::io::Error>
where
    u64: minhash_rs::primitive::Primitive<Word>,
{
    estimate_jaccard_index_minhash_for_permutation::<2, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<4, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<8, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<16, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<32, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<64, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<128, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<256, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<512, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<1024, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<2048, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<4096, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    estimate_jaccard_index_minhash_for_permutation::<8192, Word>(
        elements,
        first_set,
        second_set,
        real_jaccard,
        &file,
    )?;
    Ok(())
}

//#[test]
pub fn test_jaccard() {
    (0..1000_usize)
        .into_par_iter()
        .progress()
        .for_each(|iteration| {
            let minhash_path = format!(
                "tests/partial/test_minhash_jaccard_{iteration}.csv",
                iteration = iteration
            );
            let hll_path = format!(
                "tests/partial/test_hll_jaccard_{iteration}.csv",
                iteration = iteration
            );

            // IF the file already exists, we skip the iteration.
            if std::path::Path::new(&minhash_path).exists()
                && std::path::Path::new(&hll_path).exists()
            {
                return;
            }

            let mut minhash_file = File::create(minhash_path).unwrap();
            let mut hll_file = File::create(hll_path).unwrap();
            minhash_file
                .write_all(b"elements,permutations,word,memory,approximation,ground_truth,time\n")
                .unwrap();
            hll_file
                .write_all(b"elements,precision,bits,memory,approximation,ground_truth,time\n")
                .unwrap();
            for elements in [
                10,
                100,
                1_000,
                10_000,
                100_000,
                1_000_000,
                10_000_000,
                100_000_000,
            ]
            .iter()
            {
                let first_set = populate_set(
                    *elements,
                    (4567_u64)
                        .wrapping_mul(*elements as u64 + 1)
                        .wrapping_mul(iteration as u64 + 1),
                );
                let second_set = populate_set(
                    *elements,
                    (47325567_u64)
                        .wrapping_mul(*elements as u64 + 1)
                        .wrapping_mul(iteration as u64 + 1),
                );
                let real_jaccard = first_set.intersection(&second_set).count() as f64
                    / first_set.union(&second_set).count() as f64;

                estimate_jaccard_index_hll::<Precision4>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision5>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision6>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision7>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision8>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision9>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision10>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision11>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision12>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision13>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision14>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision15>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_hll::<Precision16>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &hll_file,
                )
                .unwrap();

                estimate_jaccard_index_minhash::<u8>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &minhash_file,
                )
                .unwrap();
                estimate_jaccard_index_minhash::<u16>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &minhash_file,
                )
                .unwrap();
                estimate_jaccard_index_minhash::<u32>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &minhash_file,
                )
                .unwrap();
                estimate_jaccard_index_minhash::<u64>(
                    *elements,
                    &first_set,
                    &second_set,
                    real_jaccard,
                    &minhash_file,
                )
                .unwrap();
            }
        });
}
