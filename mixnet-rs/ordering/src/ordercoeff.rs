use std::fmt::Display;

use protocol::node::MessageId;

#[derive(Debug, Clone)]
pub struct Sequence(Vec<Entry>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Data(MessageId),
    Noise(u32), // the number of consecutive noises
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Entry::Data(msg) => msg.to_string(),
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
        self.0.push(Entry::Data(msg));
    }

    pub fn add_noise(&mut self) {
        if let Some(last) = self.0.last_mut() {
            match last {
                Entry::Noise(cnt) => {
                    *cnt += 1;
                }
                _ => self.0.push(Entry::Noise(1)),
            }
        } else {
            self.0.push(Entry::Noise(1))
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Sequence {
    pub fn ordering_coefficient(&self, other: &Sequence, casual: bool) -> u64 {
        let mut coeff = 0;
        let mut i = 0;

        while i < self.0.len() {
            if let Entry::Data(_) = &self.0[i] {
                let (c, next_i) = self.ordering_coefficient_from(i, other, casual);
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

    fn ordering_coefficient_from(
        &self,
        start_idx: usize,
        other: &Sequence,
        casual: bool,
    ) -> (u64, usize) {
        let msg1 = match self.0[start_idx] {
            Entry::Data(msg) => msg,
            _ => panic!("Entry at {start_idx} must be Message"),
        };

        for (j, entry) in other.iter().enumerate() {
            if let Entry::Data(msg2) = entry {
                if msg1 == *msg2 {
                    // Found the 1st matching msg. Start finding the next adjacent matching msg.
                    if casual {
                        return self.casual_ordering_coefficient_from(start_idx, other, j);
                    } else {
                        return self.weak_ordering_coefficient_from(start_idx, other, j);
                    }
                }
            }
        }
        (0, start_idx)
    }

    fn casual_ordering_coefficient_from(
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

    fn weak_ordering_coefficient_from(
        &self,
        start_idx: usize,
        other: &Sequence,
        other_start_idx: usize,
    ) -> (u64, usize) {
        let mut coeff = 0;
        let mut i = start_idx + 1;
        let mut j = other_start_idx + 1;
        while i < self.0.len() && j < other.0.len() {
            i = self.skip_noise(i);
            j = other.skip_noise(j);
            if i < self.0.len() && j < other.0.len() && self.0[i] == other.0[j] {
                coeff += 1;
                i += 1;
                j += 1;
            } else {
                break;
            }
        }
        (coeff, i)
    }

    fn skip_noise(&self, mut index: usize) -> usize {
        while index < self.0.len() {
            if let Entry::Data(_) = self.0[index] {
                break;
            }
            index += 1;
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ordering_coefficient_common(casual: bool) {
        // Case 0: Empty sequences
        let seq = Sequence(vec![]);
        assert_eq!(seq.ordering_coefficient(&seq, casual), 0);

        // Case 1: Exact one matched pair with no noise
        let seq = Sequence(vec![Entry::Data(1), Entry::Data(2)]);
        assert_eq!(seq.ordering_coefficient(&seq, casual), 1);

        // Case 2: Exact one matched pair with noise
        let seq = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        assert_eq!(seq.ordering_coefficient(&seq, casual), 1);

        // Case 3: One matched pair with no noise
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Data(2), Entry::Data(3)]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Data(2), Entry::Data(4)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 1);

        // Case 4: One matched pair with noise
        let seq1 = Sequence(vec![
            Entry::Data(1),
            Entry::Noise(10),
            Entry::Data(2),
            Entry::Data(3),
        ]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 1);

        // Case 5: Two matched pairs with noise
        let seq1 = Sequence(vec![
            Entry::Data(1),
            Entry::Noise(10),
            Entry::Data(2),
            Entry::Data(3),
        ]);
        let seq2 = Sequence(vec![
            Entry::Data(1),
            Entry::Noise(10),
            Entry::Data(2),
            Entry::Data(3),
            Entry::Data(4),
        ]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 2);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 2);

        // Case 6: Only partial match with no noise
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Data(2)]);
        let seq2 = Sequence(vec![Entry::Data(2), Entry::Data(3)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 0);

        // Case 7: Only partial match with noise
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Data(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Data(2), Entry::Noise(10), Entry::Data(3)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 0);

        // Case 8: No match at all
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Data(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Data(3), Entry::Noise(10), Entry::Data(4)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 0);

        // Case 9: Matches with noise but mixed orders
        let seq1 = Sequence(vec![
            Entry::Data(1),
            Entry::Data(2),
            Entry::Noise(10),
            Entry::Data(3),
            Entry::Data(4),
            Entry::Data(5),
            Entry::Data(6),
        ]);
        let seq2 = Sequence(vec![
            Entry::Data(4),
            Entry::Data(5),
            Entry::Data(1),
            Entry::Data(2),
            Entry::Noise(10),
            Entry::Data(3),
            Entry::Data(6),
        ]);
        assert_eq!(seq1.ordering_coefficient(&seq2, casual), 3);
        assert_eq!(seq2.ordering_coefficient(&seq1, casual), 3);
    }

    #[test]
    fn test_casual_ordering_coefficient() {
        test_ordering_coefficient_common(true);

        // Case 0: No match because of noise
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Data(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, true), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, true), 0);

        // Case 1: No match because of different count of noises
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Noise(5), Entry::Data(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, true), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, true), 0);
    }

    #[test]
    fn test_weak_ordering_coefficient() {
        test_ordering_coefficient_common(false);

        // Case 0: Match ignoring noises
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Data(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, false), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, false), 1);

        // Case 1: Match ignoring noise count
        let seq1 = Sequence(vec![Entry::Data(1), Entry::Noise(10), Entry::Data(2)]);
        let seq2 = Sequence(vec![Entry::Data(1), Entry::Noise(5), Entry::Data(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, false), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, false), 1);
    }
}
