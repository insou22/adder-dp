#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}, mpsc, Mutex}, thread};

use adder_ui_model::AlgorithmProgress;

struct Global {
    progress: Arc<AtomicUsize>,
    out_of:   usize,
    receiver: mpsc::Receiver<Option<Vec<i64>>>,
}

static GLOBAL: Mutex<Option<Global>> = Mutex::new(None);

#[tauri::command]
fn run_algorithm(target: i64, number_set: Vec<i64>) {
    println!("Hello from tauri!");
    println!("target: {target}, set: {number_set:?}");

    let (sender, receiver) = mpsc::channel();
    let progress = Arc::new(AtomicUsize::new(0));
    let out_of = number_set.len();

    *GLOBAL.lock().unwrap() = Some(
        Global {
            progress: progress.clone(),
            out_of,
            receiver,
        }
    );

    thread::spawn(move || {
        let answer = adder_algorithm::run_algorithm(target, number_set, Some(progress.as_ref()));
        let _ = sender.send(answer);
    });
}

#[tauri::command]
fn check_algorithm() -> AlgorithmProgress {
    let mut lock = GLOBAL.lock().unwrap();
    let global = match lock.as_mut() {
        Some(global) => global,
        None => return AlgorithmProgress::NoAlgorithmRunning,
    };

    if let Ok(output) = global.receiver.try_recv() {
        *lock = None;
        return AlgorithmProgress::Done(output);
    }

    return AlgorithmProgress::InProgress {
        progress: global.progress.load(Ordering::SeqCst),
        out_of:   global.out_of,
    };
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_algorithm, check_algorithm])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
