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
pub enum CoefficientType {
    Strong,
    Casual,
    Weak,
}

pub fn coeff(seq1: &[Entry], seq2: &[Entry], coeff_type: CoefficientType) -> u64 {
    let mut coeff = 0;
    let mut i = 0;

    while i < seq1.len() {
        if let Entry::Data(_) = &seq1[i] {
            let (c, next_i) = coeff_from(seq1, i, seq2, coeff_type);
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
    coeff_type: CoefficientType,
) -> (u64, usize) {
    let msg1 = match seq1[start_idx] {
        Entry::Data(msg) => msg,
        _ => panic!("Entry at {start_idx} must be Message"),
    };

    for (j, entry) in seq2.iter().enumerate() {
        if let Entry::Data(msg2) = entry {
            if msg1 == *msg2 {
                // Found the 1st matching msg. Start finding the next adjacent matching msg.
                match coeff_type {
                    CoefficientType::Strong => todo!(),
                    CoefficientType::Casual => {
                        return casual_coeff_from(seq1, start_idx, seq2, j);
                    }
                    CoefficientType::Weak => {
                        return weak_coeff_from(seq1, start_idx, seq2, j);
                    }
                }
            }
        }
    }
    (0, start_idx)
}

fn casual_coeff_from(
    seq1: &[Entry],
    start_idx: usize,
    seq2: &[Entry],
    seq2_start_idx: usize,
) -> (u64, usize) {
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

fn main() {
    let seq1 = load_sequence("/Users/yjlee/repos/nomos-simulations/mixnet-rs/results/ordering_e5s1_PureCoinFlipping_2024-09-01T19:30:03.957310+00:00_0d0h12m25s/paramset_1/iteration_0_0d0h0m2s/sent_seq_0.csv");
    let seq2 = load_sequence("/Users/yjlee/repos/nomos-simulations/mixnet-rs/results/ordering_e5s1_PureCoinFlipping_2024-09-01T19:30:03.957310+00:00_0d0h12m25s/paramset_1/iteration_0_0d0h0m2s/recv_seq_0.csv");
    println!("casual:{:?}", coeff(&seq1, &seq2, CoefficientType::Casual));
    println!("weak:{:?}", coeff(&seq1, &seq2, CoefficientType::Weak));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_casual_coeff() {
        // Empty sequences
        assert_eq!(coeff(&[], &[], CoefficientType::Casual), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(coeff(&seq, &seq, CoefficientType::Casual), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(coeff(&seq, &seq, CoefficientType::Casual), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(10), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(10), data(2), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 0);

        // No match because of noise
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), data(2)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 0);
        let seq1 = vec![data(1), noise(10), data(2)];
        let seq2 = vec![data(1), noise(5), data(2)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Casual), 2);
    }

    #[test]
    fn test_weak_coeff() {
        // Empty sequences
        assert_eq!(coeff(&[], &[], CoefficientType::Weak), 0);

        // One matching pair without noise
        let seq = vec![data(1), data(2)];
        assert_eq!(coeff(&seq, &seq, CoefficientType::Weak), 1);

        // One matching pair with noise
        let seq = vec![data(1), noise(10), data(2)];
        assert_eq!(coeff(&seq, &seq, CoefficientType::Weak), 1);

        // One matching pair without noise from different sequences
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 1);
        let seq1 = vec![data(4), data(2), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 1);

        // One matching pair with noise from different sequences
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), noise(5), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 1);
        let seq1 = vec![data(4), data(2), noise(10), data(3)];
        let seq2 = vec![data(1), data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 1);

        // Two pairs with noise
        let seq1 = vec![data(1), noise(10), data(2), data(3)];
        let seq2 = vec![data(1), noise(5), data(2), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 2);

        // No match
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(2), data(3)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 0);
        let seq1 = vec![data(1), data(2)];
        let seq2 = vec![data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 0);

        // Matching pairs in different indexes
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(2), data(3), data(4), data(1)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 2);
        let seq1 = vec![data(1), data(2), data(3), data(4)];
        let seq2 = vec![data(1), data(2), data(5), data(3), data(4)];
        assert_eq!(coeff(&seq1, &seq2, CoefficientType::Weak), 2);
    }

    fn data(msg_id: u32) -> Entry {
        Entry::Data(DataMessage { sender: 0, msg_id })
    }

    fn noise(count: u32) -> Entry {
        Entry::Noise(count)
    }
}
