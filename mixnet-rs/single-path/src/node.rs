use queue::{new_queue, Queue, QueueConfig};

pub type MessageId = u32;

pub struct Node {
    queue: Box<dyn Queue<MessageId>>,
}

impl Node {
    pub fn new(queue_config: &QueueConfig) -> Self {
        Node {
            queue: new_queue(queue_config),
        }
    }

    pub fn send(&mut self, msg: MessageId) {
        // Schedule sending a new data message to the peer
        self.queue.push(msg);
    }

    pub fn receive(&mut self, msg: MessageId) {
        // Relay the message to another peer.
        // Don't need to accept noise in this function because it anyway has to be dropped.
        self.queue.push(msg);
    }

    pub fn read_queue(&mut self) -> Option<MessageId> {
        // Returns `None` if a noise was read from the queue
        self.queue.pop()
    }

    pub fn message_count_in_queue(&self) -> u32 {
        self.queue.message_count().try_into().unwrap()
    }
}
