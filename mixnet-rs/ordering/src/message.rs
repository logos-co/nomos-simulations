use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataMessage {
    pub sender: u8,
    pub msg_id: u32,
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

    pub fn next(&mut self, sender: u8) -> DataMessage {
        let msg_id = self.next_msg_ids[sender as usize];
        self.next_msg_ids[sender as usize] += 1;
        DataMessage { sender, msg_id }
    }
}
