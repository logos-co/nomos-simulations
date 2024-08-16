use std::collections::{HashMap, HashSet, VecDeque};

pub type NodeId = u16;
pub type MessageId = u32;

pub struct Node {
    id: NodeId,
    queues: HashMap<NodeId, VecDeque<MessageId>>,
    received_msgs: HashSet<MessageId>,
}

impl Node {
    pub fn new(id: NodeId) -> Self {
        Node {
            id,
            queues: HashMap::new(),
            received_msgs: HashSet::new(),
        }
    }

    pub fn connect(&mut self, peer_id: NodeId) {
        self.queues.entry(peer_id).or_default();
        tracing::info!("Node {} connected to {}", self.id, peer_id);
    }

    pub fn num_queues(&self) -> usize {
        self.queues.len()
    }

    pub fn send(&mut self, msg: MessageId) {
        assert!(self.received_msgs.insert(msg));
        for (peer_id, queue) in self.queues.iter_mut() {
            queue.push_back(msg);
            tracing::info!("Node {} sent message {} to peer {}", self.id, msg, peer_id);
        }
    }

    pub fn receive(&mut self, msg: MessageId, from: NodeId) -> bool {
        let first_received = self.received_msgs.insert(msg);
        if first_received {
            tracing::info!("Node {} received message {} from {}", self.id, msg, from);
            for (node_id, queue) in self.queues.iter_mut() {
                if *node_id != from {
                    queue.push_back(msg);
                }
            }
        } else {
            tracing::info!("Node {} ignored message {} from {}", self.id, msg, from);
        }
        first_received
    }

    pub fn read_queues(&mut self) -> Vec<(NodeId, MessageId)> {
        let mut msgs_to_relay: Vec<(NodeId, MessageId)> = Vec::new();
        for (node_id, queue) in self.queues.iter_mut() {
            if let Some(msg) = queue.pop_front() {
                msgs_to_relay.push((*node_id, msg));
            }
        }
        msgs_to_relay
    }
}
