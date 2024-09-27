use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

pub type SenderIdx = u8;

#[derive(Debug, Clone, Copy)]
pub struct DataMessage {
    pub sender: SenderIdx,
    pub msg_id: u32,
    pub num_hops_passed: u32,
}

impl DataMessage {
    pub fn increment_hops(&mut self) {
        self.num_hops_passed += 1;
    }
}

impl PartialEq for DataMessage {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender && self.msg_id == other.msg_id
    }
}

impl Eq for DataMessage {}

impl Hash for DataMessage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sender.hash(state);
        self.msg_id.hash(state);
    }
}

impl Display for DataMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.sender, self.msg_id))
    }
}

pub struct DataMessageGenerator {
    next_msg_ids: Vec<u32>,
}

impl DataMessageGenerator {
    pub fn new(num_senders: u8) -> Self {
        Self {
            next_msg_ids: vec![0; num_senders as usize],
        }
    }

    pub fn next(&mut self, sender: SenderIdx) -> DataMessage {
        let msg_id = self.next_msg_ids[sender as usize];
        self.next_msg_ids[sender as usize] += 1;
        DataMessage {
            sender,
            msg_id,
            num_hops_passed: 0,
        }
    }
}
