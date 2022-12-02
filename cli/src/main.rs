use std::io::{Write, BufRead};

fn main() {
    let Input { target, entries } = gather_input();

    match adder_algorithm::run_algorithm(target, entries, None) {
        Some(subset) => {
            println!("A correct subset:");
            for number in subset {
                println!("{number}");
            }
        }
        None => {
            println!("There is no correct subset")
        }     
    }
}

struct Input {
    target: i64,
    entries: Vec<i64>,
}

fn gather_input() -> Input {
    print!("Please enter the target in cents: ");
    std::io::stdout().flush().unwrap();
    
    let mut target = String::new();
    std::io::stdin().lock().read_line(&mut target).unwrap();
    let target = target.trim_end().parse().unwrap();

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

        entries.push(entry.trim_end().parse().unwrap());
    }

    Input {
        target,
        entries,
    }
}
