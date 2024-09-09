use ordering::message::{DataMessage, SenderIdx};

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

pub fn casual_coeff(seq1: &[Entry], seq2: &[Entry]) -> u64 {
    let mut i = 0;
    let mut j = 0;
    let mut coeff = 0;

    while i < seq1.len() && j < seq2.len() {
        let mut found_pair = false;
        if let (Entry::Data(msg1), Entry::Data(msg2)) = (&seq1[i], &seq2[j]) {
            if msg1 == msg2 {
                // Try to find the next pair of Data messages
                let mut next_i = i + 1;
                let mut next_j = j + 1;

                while next_i < seq1.len() && next_j < seq2.len() {
                    match (&seq1[next_i], &seq2[next_j]) {
                        // If there's matching noise, continue to the next element
                        (Entry::Noise(n1), Entry::Noise(n2)) => {
                            if n1 == n2 {
                                next_i += 1;
                                next_j += 1;
                            } else {
                                break;
                            }
                        }
                        // If there's a matching Data message, count the pair
                        (Entry::Data(next_msg1), Entry::Data(next_msg2)) => {
                            if next_msg1 == next_msg2 {
                                coeff += 1; // Count the adjacent pair
                                i = next_i; // Move i and j to the next DataMessage
                                j = next_j;
                                found_pair = true;
                            }
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }

        // Increment only if no matching pair was found
        if !found_pair {
            i += 1;
            j += 1;
        }
    }

    coeff
}

fn weak_coeff(seq1: &[Entry], seq2: &[Entry]) -> u64 {
    let mut i = 0;
    let mut j = 0;
    let mut coeff = 0;

    while i < seq1.len() && j < seq2.len() {
        // Skip noise in both sequences
        while i < seq1.len() && matches!(seq1[i], Entry::Noise(_)) {
            i += 1;
        }
        while j < seq2.len() && matches!(seq2[j], Entry::Noise(_)) {
            j += 1;
        }

        // Compare the DataMessages
        if i < seq1.len() && j < seq2.len() {
            if let (Entry::Data(msg1), Entry::Data(msg2)) = (&seq1[i], &seq2[j]) {
                if msg1 == msg2 {
                    // Now check the next pair
                    let mut next_i = i + 1;
                    let mut next_j = j + 1;

                    // Skip noise in both sequences for the next pair
                    while next_i < seq1.len() && matches!(seq1[next_i], Entry::Noise(_)) {
                        next_i += 1;
                    }
                    while next_j < seq2.len() && matches!(seq2[next_j], Entry::Noise(_)) {
                        next_j += 1;
                    }

                    // If the next pair of DataMessages match, count it
                    if next_i < seq1.len() && next_j < seq2.len() {
                        if let (Entry::Data(next_msg1), Entry::Data(next_msg2)) =
                            (&seq1[next_i], &seq2[next_j])
                        {
                            if next_msg1 == next_msg2 {
                                coeff += 1; // Found a matching adjacent pair
                                i = next_i;
                                j = next_j;
                                continue;
                            }
                        }
                    }
                }
            }
        }

        i += 1;
        j += 1;
    }

    coeff
}

fn main() {
    let seq1 = load_sequence("/Users/yjlee/repos/nomos-simulations/mixnet-rs/results/ordering_e5s1_PureCoinFlipping_2024-09-01T19:30:03.957310+00:00_0d0h12m25s/paramset_1/iteration_0_0d0h0m2s/sent_seq_0.csv");
    let seq2 = load_sequence("/Users/yjlee/repos/nomos-simulations/mixnet-rs/results/ordering_e5s1_PureCoinFlipping_2024-09-01T19:30:03.957310+00:00_0d0h12m25s/paramset_1/iteration_0_0d0h0m2s/recv_seq_0.csv");
    println!("casual:{:?}", casual_coeff(&seq1, &seq2));
    println!("weak:{:?}", weak_coeff(&seq1, &seq2));
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // No match because of mixed order
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(2), data(3), data(4)];
        assert_eq!(casual_coeff(&seq1, &seq2), 0);
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

        // No match because of mixed order
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![data(2), data(3), data(4)];
        assert_eq!(weak_coeff(&seq1, &seq2), 0);

        // One pair despite mixed order, because the order is mixed due to only noises.
        let seq1 = vec![data(1), data(2), data(3)];
        let seq2 = vec![noise(10), data(1), data(2)];
        assert_eq!(weak_coeff(&seq1, &seq2), 1);
    }

    fn data(msg_id: u32) -> Entry {
        Entry::Data(DataMessage { sender: 0, msg_id })
    }

    fn noise(count: u32) -> Entry {
        Entry::Noise(count)
    }
}
