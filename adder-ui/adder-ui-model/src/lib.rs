use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum AlgorithmProgress {
    NoAlgorithmRunning,
    InProgress { progress: usize, out_of: usize },
    Done(Option<Vec<i64>>),
}
