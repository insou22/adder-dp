use std::{ops::Neg, sync::atomic::{AtomicU32, Ordering, AtomicU64, AtomicUsize}};

use atomic_bitvec::AtomicBitVec;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

pub fn run_algorithm(target: i64, entries: Vec<i64>, progress: Option<&AtomicUsize>) -> Option<Vec<i64>> {
    let total = entries.len();

    let most_negative: usize = entries.iter()
        .copied()
        .filter(|x| x.is_negative())
        .map(|x| x.neg() as usize)
        .sum();

    let most_positive: usize = entries.iter()
        .copied()
        .filter(|x| x.is_positive())
        .map(|x| x as usize)
        .sum();

    let zero_index = most_negative;

    let sum_size = most_negative + 1 + most_positive;
    println!("sum_size={}", sum_size);

    let dp_table = create_dp_table(sum_size, total);

    println!("Table successfully constructed");

    for i in 0..total {
        if let Some(progress) = progress {
            progress.store(i, Ordering::SeqCst)
        }
        println!("{}/{}", i, total);
        
        let current_entry = entries[i] as isize;

        dp_table[i].set_true(zero_index);
        
        if i == 0 {
            dp_table[i].set_true((zero_index as isize + current_entry) as usize);
        } else {
            (0..sum_size).into_par_iter()
                .for_each(|j| {
                    if dp_table[i - 1].load(j) {
                        dp_table[i].set_true(j);
                    }
                });

            (0..sum_size).into_par_iter()
                .for_each(|j| {
                    let index = (j as isize) - current_entry;
                    if index < 0 {
                        return;
                    }

                    if index as usize >= sum_size {
                        return;
                    }

                    if dp_table[i - 1].load(index as usize) {
                        dp_table[i].set_true(j);
                    }
                });
        }
    }

    println!("Finished the table");
    let exists = dp_table[total - 1].load((target as isize + zero_index as isize) as usize);
    println!("Does a total of {target} exist? {exists}");

    if exists {
        let mut subset      = vec![];
        let mut current_sum = (target as isize + zero_index as isize) as usize;

        for current_i in (0..total).rev() {
            if current_i == 0 || !dp_table[current_i - 1].load(current_sum) {
                let must_include = entries[current_i];
                println!("...must include {must_include} to make sum of {}", (current_sum as isize - zero_index as isize));

                subset.push(must_include);
                current_sum = ((current_sum as isize) - (must_include as isize)) as usize;
                println!("   ...so now looking for sum of {}", (current_sum as isize - zero_index as isize));
            }

            if current_sum == zero_index { break; }
        }

        let sum: i64 = subset.iter().sum();

        println!("Sanity check: current_sum ({current_sum}) == zero_index ({zero_index})? {}", current_sum == zero_index);
        println!("Sanity check: subset sum ({sum}) == target ({target})? {}", sum == target);

        println!("Subset: {:?}", subset);

        Some(subset)
    } else {
        None
    }
}

fn create_dp_table(sum_size: usize, total: usize) -> Vec<AtomicBitVec> {
    let dp_table_progress = AtomicU32::new(0);
    (0..total).into_par_iter()
        .map(|_| {
            let mut bitvec = AtomicBitVec::with_bit_capacity(sum_size);
            bitvec.resize_bits_with(sum_size, || AtomicU64::new(0));
            bitvec
        })
        .inspect(|_| println!("{}/{total}", dp_table_progress.fetch_add(1, Ordering::SeqCst) + 1))
        .collect::<Vec<_>>()
}

trait AtomicBitVecExt {
    fn load(&self, index: usize) -> bool;
    fn set_true(&self, index: usize);
    fn set_false(&self, index: usize);
}

impl AtomicBitVecExt for AtomicBitVec {
    fn load(&self, index: usize) -> bool {
        self.get(index, Ordering::SeqCst)
    }

    fn set_true(&self, index: usize) {
        self.set(index, true, Ordering::SeqCst);
    }

    fn set_false(&self, index: usize) {
        self.set(index, false, Ordering::SeqCst);
    }
}
