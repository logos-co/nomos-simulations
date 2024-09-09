use std::{fs::File, path::PathBuf};

use clap::Parser;
use glob::glob;
use polars::prelude::*;
use walkdir::WalkDir;

use ordering::message::{DataMessage, SenderIdx};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Data(DataMessage),
    Noise(u32), // the number of consecutive noises
}

fn load_sequence(path: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .unwrap();

    for result in reader.records() {
        let record = result.unwrap();
        let value = &record[0];

        if let Ok(num) = value.parse::<i32>() {
            assert!(num < 0);
            entries.push(Entry::Noise(num.unsigned_abs()));
        } else {
            entries.push(Entry::Data(parse_data_msg(value)));
        }
    }

    entries
}

fn parse_data_msg(value: &str) -> DataMessage {
    let parts: Vec<&str> = value.split(':').collect();
    assert_eq!(parts.len(), 2);
    DataMessage {
        sender: parts[0].parse::<SenderIdx>().unwrap(),
        msg_id: parts[1].parse::<u32>().unwrap(),
    }
}

pub fn strong_and_casual_coeff(seq1: &[Entry], seq2: &[Entry]) -> (u64, u64) {
    let coeffs = coeff(seq1, seq2, CoefficientType::StrongAndCasual);
    assert_eq!(coeffs.len(), 2);
    (coeffs[0], coeffs[1])
}

pub fn weak_coeff(seq1: &[Entry], seq2: &[Entry]) -> u64 {
    let coeffs = coeff(seq1, seq2, CoefficientType::Weak);
    assert_eq!(coeffs.len(), 1);
    coeffs[0]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoefficientType {
    StrongAndCasual,
    Weak,
}

impl CoefficientType {
    fn zero(&self) -> Vec<u64> {
        match self {
            CoefficientType::StrongAndCasual => vec![0, 0],
            CoefficientType::Weak => vec![0],
        }
    }
}

fn coeff(seq1: &[Entry], seq2: &[Entry], coeff_type: CoefficientType) -> Vec<u64> {
    let mut coeffs = coeff_type.zero();
    let mut i = 0;

    while i < seq1.len() {
        if let Entry::Data(_) = &seq1[i] {
            let (c, next_i) = coeff_from(seq1, i, seq2, coeff_type);
            assert_eq!(coeffs.len(), c.len());
            coeffs = coeffs
                .iter()
                .zip(c.iter())
                .map(|(a, b)| a.checked_add(*b).unwrap())
                .collect();

            if next_i != i {
                i = next_i;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    coeffs
}

fn coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    coeff_type: CoefficientType,
) -> (Vec<u64>, usize) {
    let msg1 = match seq1[start_idx] {
        Entry::Data(msg) => msg,
        _ => panic!("Entry at index {start_idx} must be Message"),
    };

    for (j, entry) in seq2.iter().enumerate() {
        if let Entry::Data(msg2) = entry {
            if msg1 == *msg2 {
                // Found the 1st matching msg. Start finding the next adjacent matching msg.
                match coeff_type {
                    CoefficientType::StrongAndCasual => {
                        return strong_and_casual_coeff_from(seq1, start_idx, seq2, j);
                    }
                    CoefficientType::Weak => {
                        let (coeff, next_start_idx) = weak_coeff_from(seq1, start_idx, seq2, j);
                        return (vec![coeff], next_start_idx);
                    }
                }
            }
        }
    }

    // Couldn't find any matching msg in seq2. Returning the zero coefficients and the same start index.
    (coeff_type.zero(), start_idx)
}

fn strong_and_casual_coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    seq2_start_idx: usize,
) -> (Vec<u64>, usize) {
    let (casual_coeff, next_start_idx) = casual_coeff_from(seq1, start_idx, seq2, seq2_start_idx);

    // The partially calcuated casual coefficient means the number of consecutive matching pairs,
    // which is also what should be calculated for the strong coefficient.
    // So, we reuse it.
    let num_consecutive_pairs = casual_coeff;
    let strong_coeff = if num_consecutive_pairs == 0 {
        0
    } else {
        (num_consecutive_pairs)
            .checked_pow(num_consecutive_pairs.try_into().unwrap())
            .unwrap()
    };
    (vec![strong_coeff, casual_coeff], next_start_idx)
}

fn casual_coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    seq2_start_idx: usize,
) -> (u64, usize) {
    // Find the number of consecutive matching pairs while accounting for noises.
    let mut coeff = 0;
    let mut i = start_idx + 1;
    let mut j = seq2_start_idx + 1;
    while i < seq1.len() && j < seq2.len() {
        match (&seq1[i], &seq2[j]) {
            (Entry::Noise(cnt1), Entry::Noise(cnt2)) => {
                if cnt1 == cnt2 {
                    i += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            (Entry::Data(msg1), Entry::Data(msg2)) => {
                if msg1 == msg2 {
                    coeff += 1;
                    i += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    (coeff, i)
}

fn weak_coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    seq2_start_idx: usize,
) -> (u64, usize) {
    // Find the number of consecutive matching pairs with ignoring noises.
    let mut coeff = 0;
    let mut i = start_idx + 1;
    let mut j = seq2_start_idx + 1;
    while i < seq1.len() && j < seq2.len() {
        i = skip_noise(seq1, i);
        j = skip_noise(seq2, j);
        if i < seq1.len() && j < seq2.len() && seq1[i] == seq2[j] {
            coeff += 1;
            i += 1;
            j += 1;
        } else {
            break;
        }
    }
    (coeff, i)
}

fn skip_noise(seq: &[Entry], mut index: usize) -> usize {
    while index < seq.len() {
        if let Entry::Data(_) = seq[index] {
            break;
        }
        index += 1;
    }
    index
}

#[derive(Debug, Parser)]
#[command(name = "Calculating ordering coefficients")]
struct Args {
    #[arg(short, long)]
    path: String,
    #[arg(short, long)]
    num_threads: usize,
}

fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    calculate_coeffs(&args);
}

fn calculate_coeffs(args: &Args) {
    let mut tasks: Vec<Task> = Vec::new();
    for entry in WalkDir::new(args.path.as_str())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.path().file_name().unwrap().to_string_lossy();
        if dir_name.starts_with("iteration_") {
            for sent_seq_file in glob(&format!("{}/sent_seq_*.csv", entry.path().display()))
                .unwrap()
                .filter_map(Result::ok)
            {
                let sender =
                    extract_id(&sent_seq_file.file_name().unwrap().to_string_lossy()).unwrap();

                for recv_seq_file in glob(&format!("{}/recv_seq_*.csv", entry.path().display()))
                    .unwrap()
                    .filter_map(Result::ok)
                {
                    let receiver =
                        extract_id(&recv_seq_file.file_name().unwrap().to_string_lossy()).unwrap();

                    let task = Task {
                        sent_seq_file: sent_seq_file.clone(),
                        recv_seq_file: recv_seq_file.clone(),
                        sender,
                        receiver,
                        outpath: entry
                            .path()
                            .join(format!("coeffs_{}_{}.csv", sender, receiver)),
                    };
                    tasks.push(task);
                }
            }
        }
    }

    let (task_tx, task_rx) = crossbeam::channel::unbounded::<Task>();
    let mut threads = Vec::with_capacity(args.num_threads);
    for _ in 0..args.num_threads {
        let task_rx = task_rx.clone();

        let thread = std::thread::spawn(move || {
            while let Ok(task) = task_rx.recv() {
                task.run();
            }
        });
        threads.push(thread);
    }

    for task in tasks {
        task_tx.send(task).unwrap();
    }
    // Close the task sender channel, so that the threads can know that there's no task remains.
    drop(task_tx);

    for thread in threads {
        thread.join().unwrap();
    }
}

fn extract_id(filename: &str) -> Option<u8> {
    if let Some(stripped) = filename.strip_suffix(".csv") {
        if let Some(stripped) = stripped.strip_prefix("sent_seq_") {
            return stripped.parse::<u8>().ok();
        } else if let Some(stripped) = stripped.strip_prefix("recv_seq_") {
            return stripped.parse::<u8>().ok();
        }
    }
    None
}

struct Task {
    sent_seq_file: PathBuf,
    recv_seq_file: PathBuf,
    sender: u8,
    receiver: u8,
    outpath: PathBuf,
}

impl Task {
    fn run(&self) {
        tracing::info!(
            "Processing:\n  {}\n  {}",
            self.sent_seq_file.display(),
            self.recv_seq_file.display()
        );

        let sent_seq = load_sequence(self.sent_seq_file.to_str().unwrap());
        let recv_seq = load_sequence(self.recv_seq_file.to_str().unwrap());
        let (strong, casual) = strong_and_casual_coeff(&sent_seq, &recv_seq);
        let weak = weak_coeff(&sent_seq, &recv_seq);

        let mut df = DataFrame::new(vec![
            Series::new("sender", &[self.sender as u64]),
            Series::new("receiver", &[self.receiver as u64]),
            Series::new("strong", &[strong]),
            Series::new("casual", &[casual]),
            Series::new("weak", &[weak]),
        ])
        .unwrap()
        .sort(["sender", "receiver"], SortMultipleOptions::default())
        .unwrap();
        let mut file = File::create(&self.outpath).unwrap();
        CsvWriter::new(&mut file).finish(&mut df).unwrap();
        tracing::info!("Saved {}", self.outpath.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_coeff() {
        // Empty sequences
        assert_eq!(strong_coeff(&[], &[]), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(strong_coeff(&seq, &seq), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(strong_coeff(&seq, &seq), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(strong_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(strong_coeff(&seq1, &seq2), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(4)];
        assert_eq!(strong_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(10), data(3)];
        assert_eq!(strong_coeff(&seq1, &seq2), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(3), data(4)];
        assert_eq!(strong_coeff(&seq1, &seq2), 4);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(strong_coeff(&seq1, &seq2), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(strong_coeff(&seq1, &seq2), 0);

        // No match because of noise
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), data(2)];
        assert_eq!(strong_coeff(&seq1, &seq2), 0);
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), noise(5), data(2)];
        assert_eq!(strong_coeff(&seq1, &seq2), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(strong_coeff(&seq1, &seq2), 4);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(strong_coeff(&seq1, &seq2), 2);
    }

    #[test]
    fn test_casual_coeff() {
        // Empty sequences
        assert_eq!(casual_coeff(&[], &[]), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(casual_coeff(&seq, &seq), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(casual_coeff(&seq, &seq), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(casual_coeff(&seq1, &seq2), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(10), data(3)];
        assert_eq!(casual_coeff(&seq1, &seq2), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(3), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(casual_coeff(&seq1, &seq2), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 0);

        // No match because of noise
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), data(2)];
        assert_eq!(casual_coeff(&seq1, &seq2), 0);
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), noise(5), data(2)];
        assert_eq!(casual_coeff(&seq1, &seq2), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(casual_coeff(&seq1, &seq2), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 2);
    }

    #[test]
    fn test_weak_coeff() {
        // Empty sequences
        assert_eq!(weak_coeff(&[], &[]), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(weak_coeff(&seq, &seq), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(weak_coeff(&seq, &seq), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(5), data(3)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(3), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(weak_coeff(&seq1, &seq2), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(weak_coeff(&seq1, &seq2), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 2);
    }

    fn data(msg_id: u32) -> Entry {
        Entry::Data(DataMessage { sender: 0, msg_id })
    }

    fn noise(count: u32) -> Entry {
        Entry::Noise(count)
    }

    fn strong_coeff(seq1: &[Entry], seq2: &[Entry]) -> u64 {
        let (strong, _) = strong_and_casual_coeff(seq1, seq2);
        strong
    }

    fn casual_coeff(seq1: &[Entry], seq2: &[Entry]) -> u64 {
        let (_, casual) = strong_and_casual_coeff(seq1, seq2);
        casual
    }
}
