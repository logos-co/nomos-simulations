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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderingType {
    Strong,
    Casual,
    Weak,
}

fn coeff(seq1: &[Entry], seq2: &[Entry], ordering_type: OrderingType) -> u64 {
    let mut coeff = 0;
    let mut i = 0;

    while i < seq1.len() {
        if let Entry::Data(_) = &seq1[i] {
            let (c, next_i) = coeff_from(seq1, i, seq2, ordering_type);
            coeff += c;

            if next_i != i {
                i = next_i;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    coeff
}

fn coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    ordering_type: OrderingType,
) -> (u64, usize) {
    let msg1 = match seq1[start_idx] {
        Entry::Data(msg) => msg,
        _ => panic!("Entry at index {start_idx} must be Message"),
    };

    for (j, entry) in seq2.iter().enumerate() {
        if let Entry::Data(msg2) = entry {
            if msg1 == *msg2 {
                // Found the 1st matching msg. Start finding the next adjacent matching msg.
                match ordering_type {
                    OrderingType::Strong => {
                        return strong_coeff_from(seq1, start_idx, seq2, j);
                    }
                    OrderingType::Casual => {
                        return casual_coeff_from(seq1, start_idx, seq2, j);
                    }
                    OrderingType::Weak => {
                        return weak_coeff_from(seq1, start_idx, seq2, j);
                    }
                }
            }
        }
    }

    // Couldn't find any matching msg in seq2. Returning the zero coefficients and the same start index.
    (0, start_idx)
}

fn strong_coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    seq2_start_idx: usize,
) -> (u64, usize) {
    // Find the number of consecutive matching exactly pairs that don't contain noises.
    let mut num_consecutive_pairs: u64 = 0;
    let mut i = start_idx + 1;
    let mut j = seq2_start_idx + 1;
    while i < seq1.len() && j < seq2.len() {
        match (&seq1[i], &seq2[j]) {
            (Entry::Data(msg1), Entry::Data(msg2)) => {
                if msg1 == msg2 {
                    num_consecutive_pairs += 1;
                    i += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }

    let coeff = if num_consecutive_pairs == 0 {
        0
    } else {
        num_consecutive_pairs
            .checked_pow(num_consecutive_pairs.try_into().unwrap())
            .unwrap()
    };
    (coeff, i)
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
        let strong = coeff(&sent_seq, &recv_seq, OrderingType::Strong);
        let casual = coeff(&sent_seq, &recv_seq, OrderingType::Casual);
        let weak = coeff(&sent_seq, &recv_seq, OrderingType::Weak);

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
        assert_eq!(coeff(&[], &[], OrderingType::Strong), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Strong), 1);

        // No matching pair due to noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Strong), 0);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 1);

        // One pair, not two because of noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 1);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 4);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Strong), 2);
    }

    #[test]
    fn test_casual_coeff() {
        // Empty sequences
        assert_eq!(coeff(&[], &[], OrderingType::Casual), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Casual), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Casual), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(10), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 0);

        // No match because of noise
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), data(2)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 0);
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), noise(5), data(2)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Casual), 2);
    }

    #[test]
    fn test_weak_coeff() {
        // Empty sequences
        assert_eq!(coeff(&[], &[], OrderingType::Weak), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Weak), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(coeff(&seq, &seq, OrderingType::Weak), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(5), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, OrderingType::Weak), 2);
    }

    fn data(msg_id: u32) -> Entry {
        Entry::Data(DataMessage { sender: 0, msg_id })
    }

    fn noise(count: u32) -> Entry {
        Entry::Noise(count)
    }
}
