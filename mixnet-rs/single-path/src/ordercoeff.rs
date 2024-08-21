use std::fmt::Display;

use crate::node::MessageId;

pub struct Sequence(Vec<Entry>);

#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Message(MessageId),
    Noise(u32), // the number of consecutive noises
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Entry::Message(msg) => msg.to_string(),
            Entry::Noise(cnt) => format!("-{cnt}"),
        };
        f.write_str(s.as_str())
    }
}

impl Sequence {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_message(&mut self, msg: MessageId) {
        self.0.push(Entry::Message(msg));
    }

    pub fn add_noise(&mut self) {
        if let Some(last) = self.0.last_mut() {
            if let Entry::Noise(cnt) = last {
                *cnt += 1;
            } else {
                self.0.push(Entry::Noise(1))
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.0.iter()
    }
}

impl Sequence {
    pub fn calculate_strong_ordering_coefficient(&self, other: &Sequence) -> u64 {
        let mut coeff = 0;
        let mut i = 0;

        while i < self.0.len() {
            if let Entry::Message(_) = &self.0[i] {
                let (c, next_i) = self.calculate_strong_ordering_coefficient_from(i, other);
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

    fn calculate_strong_ordering_coefficient_from(
        &self,
        start_idx: usize,
        other: &Sequence,
    ) -> (u64, usize) {
        let msg1 = match self.0[start_idx] {
            Entry::Message(msg) => msg,
            _ => panic!("Entry at {start_idx} must be Message"),
        };

        for (j, entry) in other.iter().enumerate() {
            if let Entry::Message(msg2) = entry {
                if msg1 == *msg2 {
                    // Found the 1st matching msg. Start finding the next adjacent matching msg.
                    return self.scan_adjacent_common_msgs(start_idx, other, j);
                }
            }
        }
        (0, start_idx)
    }

    fn scan_adjacent_common_msgs(
        &self,
        start_idx: usize,
        other: &Sequence,
        other_start_idx: usize,
    ) -> (u64, usize) {
        let mut coeff = 0;
        let mut i = start_idx + 1;
        let mut j = other_start_idx + 1;
        while i < self.0.len() && j < other.0.len() {
            match (&self.0[i], &other.0[j]) {
                (Entry::Noise(cnt1), Entry::Noise(cnt2)) => {
                    if cnt1 == cnt2 {
                        i += 1;
                        j += 1;
                    } else {
                        break;
                    }
                }
                (Entry::Message(msg1), Entry::Message(msg2)) => {
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

    // pub fn calculate_weak_ordering_coefficient(&self, other: &Sequence) -> u64 {
    //     let mut i = 0;
    //     let mut j = 0;
    //     let mut coeff = 0;

    //     while i < self.0.len() && j < other.0.len() {
    //         // Skip noise to find the first message in both sequences
    //         i = self.skip_noise(i);
    //         j = other.skip_noise(j);

    //         // Check if both indices point to valid messages and match
    //         if i < self.0.len() && j < other.0.len() && self.0[i] == other.0[j] {
    //             let next_i = self.skip_noise(i + 1);
    //             let next_j = other.skip_noise(j + 1);

    //             // Check if the next pair of messages match
    //             if next_i < self.0.len()
    //                 && next_j < other.0.len()
    //                 && self.0[next_i] == other.0[next_j]
    //             {
    //                 coeff += 1;
    //             }

    //             // Move to the next message pair
    //             i = next_i;
    //             j = next_j;
    //         } else {
    //             // Move to the next elements in the sequences
    //             i += 1;
    //             // j += 1;
    //         }
    //     }

    //     coeff
    // }

    // fn skip_noise(&self, mut index: usize) -> usize {
    //     while index < self.0.len() {
    //         if let Entry::Message(_) = self.0[index] {
    //             break;
    //         }
    //         index += 1;
    //     }
    //     index
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_ordering_coefficient() {
        // Case 0: Empty sequences
        let seq = Sequence(vec![]);
        assert_eq!(seq.calculate_strong_ordering_coefficient(&seq), 0);

        // Case 1: Exact one matched pair with no noise
        let seq = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        assert_eq!(seq.calculate_strong_ordering_coefficient(&seq), 1);

        // Case 2: Exact one matched pair with noise
        let seq = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        assert_eq!(seq.calculate_strong_ordering_coefficient(&seq), 1);

        // Case 3: One matched pair with no noise
        let seq1 = Sequence(vec![
            Entry::Message(1),
            Entry::Message(2),
            Entry::Message(3),
        ]);
        let seq2 = Sequence(vec![
            Entry::Message(1),
            Entry::Message(2),
            Entry::Message(4),
        ]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 1);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 1);

        // Case 4: One matched pair with noise
        let seq1 = Sequence(vec![
            Entry::Message(1),
            Entry::Noise(10),
            Entry::Message(2),
            Entry::Message(3),
        ]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 1);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 1);

        // Case 5: Two matched pairs with noise
        let seq1 = Sequence(vec![
            Entry::Message(1),
            Entry::Noise(10),
            Entry::Message(2),
            Entry::Message(3),
        ]);
        let seq2 = Sequence(vec![
            Entry::Message(1),
            Entry::Noise(10),
            Entry::Message(2),
            Entry::Message(3),
            Entry::Message(4),
        ]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 2);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 2);

        // Case 6: Only partial match with no noise
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(2), Entry::Message(3)]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 0);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 0);

        // Case 7: Only partial match with noise
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Message(2), Entry::Noise(10), Entry::Message(3)]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 0);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 0);

        // Case 8: No match at all
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Message(3), Entry::Noise(10), Entry::Message(4)]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 0);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 0);

        // Case 9: No match because of different count of noises
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Noise(5), Entry::Message(2)]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 0);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 0);

        // Case 9: Matches with noise but mixed orders
        let seq1 = Sequence(vec![
            Entry::Message(1),
            Entry::Message(2),
            Entry::Noise(10),
            Entry::Message(3),
            Entry::Message(4),
            Entry::Message(5),
            Entry::Message(6),
        ]);
        let seq2 = Sequence(vec![
            Entry::Message(4),
            Entry::Message(5),
            Entry::Message(1),
            Entry::Message(2),
            Entry::Noise(10),
            Entry::Message(3),
            Entry::Message(6),
        ]);
        assert_eq!(seq1.calculate_strong_ordering_coefficient(&seq2), 3);
        assert_eq!(seq2.calculate_strong_ordering_coefficient(&seq1), 3);
    }
}
