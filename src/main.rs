use std::{io::{BufRead, Write}, ops::{Neg, Not}, sync::atomic::{AtomicU32, Ordering}, fmt::Display};

use rayon::prelude::{IntoParallelIterator, ParallelIterator};

fn main() {
    let Input {
        total: target,
        entries,
    } = gather_input();

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

    let mut dp_table = create_dp_table(sum_size, total);

    println!("Table successfully constructed");

    for i in 0..total {
        println!("{}/{}", i, total);
        
        let current_entry = entries[i] as isize;

        dp_table[i][zero_index] = AtomicBool::new(true);
        
        if i == 0 {
            dp_table[i][(zero_index as isize + current_entry) as usize].set_true();
        } else {
            (0..sum_size).into_par_iter()
                .for_each(|j| {
                    if dp_table[i - 1][j].load() {
                        dp_table[i][j].set_true();
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

                    if dp_table[i - 1][index as usize].load() {
                        dp_table[i][j].set_true();
                    }
                });
        }
    }

    println!("Finished the table");
    let exists = dp_table[total - 1][(target as isize + zero_index as isize) as usize].load();
    println!("Does a total of {target} exist? {exists}");

    if exists {
        let mut subset      = vec![];
        let mut current_sum = (target as isize + zero_index as isize) as usize;

        for current_i in (0..total).rev() {
            if current_i == 0 || !dp_table[current_i - 1][current_sum].load() {
                let must_include = entries[current_i];
                println!("...must include {must_include} to make sum of {}", (current_sum as isize - zero_index as isize));

                subset.push(must_include);
                current_sum = ((current_sum as isize) - (must_include as isize)) as usize;
                println!("   ...so now looking for sum of {}", (current_sum as isize - zero_index as isize));
            }

            if current_sum == zero_index { break; }
        }

        let sum: i32 = subset.iter().sum();

        println!("Sanity check: current_sum ({current_sum}) == zero_index ({zero_index})? {}", current_sum == zero_index);
        println!("Sanity check: subset sum ({sum}) == target ({target})? {}", sum == target);

        println!("Subset: {:?}", subset);
    }
}

fn create_dp_table(sum_size: usize, total: usize) -> Vec<Vec<AtomicBool>> {
    let mut last = Vec::with_capacity(sum_size);
    for _ in 0..sum_size {
        last.push(AtomicBool::new(false));
    }
    let dp_table_progress = AtomicU32::new(0);
    let mut dp_table = (1..total).into_par_iter()
        .map(|_| last.clone())
        .inspect(|_| println!("{}/{total}", dp_table_progress.fetch_add(1, Ordering::SeqCst) + 1))
        .collect::<Vec<_>>();
    dp_table.push(last);
    println!("{total}/{total}");

    dp_table
}


#[repr(transparent)]
struct AtomicBool {
    inner: std::sync::atomic::AtomicBool,
}

impl AtomicBool {
    fn new(b: bool) -> Self {
        Self { inner: std::sync::atomic::AtomicBool::new(b) }
    }

    fn set_true(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    fn load(&self) -> bool {
        self.into()
    }
}

impl Clone for AtomicBool {
    fn clone(&self) -> Self {
        Self::new(self.inner.load(Ordering::SeqCst))
    }
}

impl PartialEq<bool> for AtomicBool {
    fn eq(&self, other: &bool) -> bool {
        self.inner.load(Ordering::SeqCst) == *other
    }
}

impl Not for AtomicBool {
    type Output = bool;

    fn not(self) -> Self::Output {
        !self.inner.load(Ordering::SeqCst)
    }
}

impl Display for AtomicBool {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.inner.load(Ordering::SeqCst).fmt(f)
    }
}

impl From<&AtomicBool> for bool {
    fn from(b: &AtomicBool) -> bool {
        b.inner.load(Ordering::SeqCst)
    }
}


struct Input {
    total: i32,
    entries: Vec<i32>,
}

fn gather_input() -> Input {
    print!("Please enter the total in cents: ");
    std::io::stdout().flush().unwrap();
    
    let mut total = String::new();
    std::io::stdin().lock().read_line(&mut total).unwrap();
    let total = total.trim_end().parse::<i32>().unwrap();

    print!("Please enter the number of entries: ");
    std::io::stdout().flush().unwrap();

    let mut n_entries = String::new();
    std::io::stdin().lock().read_line(&mut n_entries).unwrap();
    let n_entries = n_entries.trim_end().parse::<usize>().unwrap();

    println!("Please enter the {} entries in cents (1 per line): ", n_entries);
    let mut entries = Vec::with_capacity(n_entries);
    for _ in 0..n_entries {
        let mut entry = String::new();
        std::io::stdin().lock().read_line(&mut entry).unwrap();

        entries.push(entry.trim_end().parse::<i32>().unwrap());
    }

    Input {
        total,
        entries,
    }
}
