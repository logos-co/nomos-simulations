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
    pub fn ordering_coefficient(&self, other: &Sequence, strong: bool) -> u64 {
        let mut coeff = 0;
        let mut i = 0;

        while i < self.0.len() {
            if let Entry::Message(_) = &self.0[i] {
                let (c, next_i) = self.ordering_coefficient_from(i, other, strong);
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
        strong: bool,
    ) -> (u64, usize) {
        let msg1 = match self.0[start_idx] {
            Entry::Message(msg) => msg,
            _ => panic!("Entry at {start_idx} must be Message"),
        };

        for (j, entry) in other.iter().enumerate() {
            if let Entry::Message(msg2) = entry {
                if msg1 == *msg2 {
                    // Found the 1st matching msg. Start finding the next adjacent matching msg.
                    if strong {
                        return self.strong_ordering_coefficient_from(start_idx, other, j);
                    } else {
                        return self.weak_ordering_coefficient_from(start_idx, other, j);
                    }
                }
            }
        }
        (0, start_idx)
    }

    fn strong_ordering_coefficient_from(
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
            if let Entry::Message(_) = self.0[index] {
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

    fn test_ordering_coefficient_common(strong: bool) {
        // Case 0: Empty sequences
        let seq = Sequence(vec![]);
        assert_eq!(seq.ordering_coefficient(&seq, strong), 0);

        // Case 1: Exact one matched pair with no noise
        let seq = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        assert_eq!(seq.ordering_coefficient(&seq, strong), 1);

        // Case 2: Exact one matched pair with noise
        let seq = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        assert_eq!(seq.ordering_coefficient(&seq, strong), 1);

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
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 1);

        // Case 4: One matched pair with noise
        let seq1 = Sequence(vec![
            Entry::Message(1),
            Entry::Noise(10),
            Entry::Message(2),
            Entry::Message(3),
        ]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 1);

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
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 2);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 2);

        // Case 6: Only partial match with no noise
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(2), Entry::Message(3)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 0);

        // Case 7: Only partial match with noise
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Message(2), Entry::Noise(10), Entry::Message(3)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 0);

        // Case 8: No match at all
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Message(2), Entry::Noise(10)]);
        let seq2 = Sequence(vec![Entry::Message(3), Entry::Noise(10), Entry::Message(4)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 0);

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
        assert_eq!(seq1.ordering_coefficient(&seq2, strong), 3);
        assert_eq!(seq2.ordering_coefficient(&seq1, strong), 3);
    }

    #[test]
    fn test_strong_ordering_coefficient() {
        test_ordering_coefficient_common(true);

        // Case 0: No match because of noise
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, true), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, true), 0);

        // Case 1: No match because of different count of noises
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Noise(5), Entry::Message(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, true), 0);
        assert_eq!(seq2.ordering_coefficient(&seq1, true), 0);
    }

    #[test]
    fn test_weak_ordering_coefficient() {
        test_ordering_coefficient_common(false);

        // Case 0: Match ignoring noises
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Message(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, false), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, false), 1);

        // Case 1: Match ignoring noise count
        let seq1 = Sequence(vec![Entry::Message(1), Entry::Noise(10), Entry::Message(2)]);
        let seq2 = Sequence(vec![Entry::Message(1), Entry::Noise(5), Entry::Message(2)]);
        assert_eq!(seq1.ordering_coefficient(&seq2, false), 1);
        assert_eq!(seq2.ordering_coefficient(&seq1, false), 1);
    }
}
