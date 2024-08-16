use std::collections::{HashMap, HashSet};

use crate::queue::{new_queue, Queue, QueueConfig};

pub type NodeId = u16;
pub type MessageId = u32;

pub struct Node {
    queue_config: QueueConfig,
    queues: HashMap<NodeId, Box<dyn Queue<MessageId>>>,
    received_msgs: HashSet<MessageId>,
}

impl Node {
    pub fn new(queue_config: QueueConfig) -> Self {
        Node {
            queue_config,
            queues: HashMap::new(),
            received_msgs: HashSet::new(),
        }
    }

    pub fn connect(&mut self, peer_id: NodeId) {
        self.queues
            .entry(peer_id)
            .or_insert(new_queue(&self.queue_config));
    }

    pub fn num_queues(&self) -> usize {
        self.queues.len()
    }

    pub fn send(&mut self, msg: MessageId) {
        assert!(self.received_msgs.insert(msg));
        for (_, queue) in self.queues.iter_mut() {
            queue.push(msg);
        }
    }

    pub fn receive(&mut self, msg: MessageId, from: NodeId) -> bool {
        let first_received = self.received_msgs.insert(msg);
        if first_received {
            for (node_id, queue) in self.queues.iter_mut() {
                if *node_id != from {
                    queue.push(msg);
                }
            }
        }
        first_received
    }

    pub fn read_queues(&mut self) -> Vec<(NodeId, MessageId)> {
        let mut msgs_to_relay: Vec<(NodeId, MessageId)> = Vec::new();
        for (node_id, queue) in self.queues.iter_mut() {
            if let Some(msg) = queue.pop() {
                msgs_to_relay.push((*node_id, msg));
            }
        }
        msgs_to_relay
    }
}
