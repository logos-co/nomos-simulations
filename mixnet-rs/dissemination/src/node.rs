use rustc_hash::{FxHashMap, FxHashSet};

use crate::queue::{new_queue, Queue, QueueConfig};

pub type NodeId = u32;
pub type MessageId = u32;

pub struct Node {
    queue_config: QueueConfig,
    // To have the deterministic result, we use Vec instead of FxHashMap.
    // Building `queues` is inefficient, but it's not a problem because it's done only once at the beginning.
    // Instead, use `connected_peers` to build `queues` efficiently.
    queues: Vec<(NodeId, Box<dyn Queue<MessageId>>)>,
    connected_peers: FxHashSet<NodeId>,
    // A cache to avoid relaying the same message multiple times.
    received_msgs: FxHashMap<MessageId, u32>,
    peering_degree: u32,
}

impl Node {
    pub fn new(queue_config: QueueConfig, peering_degree: u32) -> Self {
        Node {
            queue_config,
            queues: Vec::new(),
            connected_peers: FxHashSet::default(),
            received_msgs: FxHashMap::default(),
            peering_degree,
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
        assert!(self.check_and_update_cache(msg, true));
        for (_, queue) in self.queues.iter_mut() {
            queue.push(msg);
        }
    }

    pub fn receive(&mut self, msg: MessageId, from: NodeId) -> bool {
        let first_received = self.check_and_update_cache(msg, false);
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

    fn check_and_update_cache(&mut self, msg: MessageId, sending: bool) -> bool {
        let first_received = if let Some(count) = self.received_msgs.get_mut(&msg) {
            *count += 1;
            false
        } else {
            self.received_msgs.insert(msg, if sending { 0 } else { 1 });
            true
        };

        // If the message have been received from all connected peers, remove it from the cache
        // because there is no possibility that the message will be received again.
        if self.received_msgs.get(&msg).unwrap() == &self.peering_degree {
            tracing::debug!("Remove message from cache: {}", msg);
            self.received_msgs.remove(&msg);
        }

        first_received
    }
}
