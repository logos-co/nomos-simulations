use std::collections::HashSet;

use crate::queue::{new_queue, Queue, QueueConfig};

pub type NodeId = u16;
pub type MessageId = u32;

pub struct Node {
    queue_config: QueueConfig,
    // To have the deterministic result, we use Vec instead of HashMap.
    // Building `queues` is inefficient, but it's not a problem because it's done only once at the beginning.
    // Instead, use `connected_peers` to build `queues` efficiently.
    queues: Vec<(NodeId, Box<dyn Queue<MessageId>>)>,
    connected_peers: HashSet<NodeId>,
    // A cache to avoid relaying the same message multiple times.
    received_msgs: HashSet<MessageId>,
}

impl Node {
    pub fn new(queue_config: QueueConfig) -> Self {
        Node {
            queue_config,
            queues: Vec::new(),
            connected_peers: HashSet::new(),
            received_msgs: HashSet::new(),
        }
    }

    pub fn connect(&mut self, peer_id: NodeId) {
        if self.connected_peers.insert(peer_id) {
            let pos = self
                .queues
                .binary_search_by(|probe| probe.0.cmp(&peer_id))
                .unwrap_or_else(|pos| pos);
            self.queues
                .insert(pos, (peer_id, new_queue(&self.queue_config)));
        }
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
