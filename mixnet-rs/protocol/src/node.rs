use rustc_hash::{FxHashMap, FxHashSet};

use crate::queue::{new_queue, Message, Queue, QueueConfig};

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
    received_msgs: Option<FxHashMap<MessageId, u32>>,
    peering_degree: u32,
}

impl Node {
    pub fn new(queue_config: QueueConfig, peering_degree: u32, enable_cache: bool) -> Self {
        Node {
            queue_config,
            queues: Vec::new(),
            connected_peers: FxHashSet::default(),
            received_msgs: if enable_cache {
                Some(FxHashMap::default())
            } else {
                None
            },
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

    pub fn receive(&mut self, msg: MessageId, from: Option<NodeId>) -> bool {
        let first_received = self.check_and_update_cache(msg, false);
        if first_received {
            for (node_id, queue) in self.queues.iter_mut() {
                match from {
                    Some(sender) => {
                        if *node_id != sender {
                            queue.push(msg);
                        }
                    }
                    None => queue.push(msg),
                }
            }
        }
        first_received
    }

    pub fn read_queues(&mut self) -> Vec<(NodeId, Message<MessageId>)> {
        let mut msgs_to_relay: Vec<(NodeId, Message<MessageId>)> = Vec::new();
        self.queues.iter_mut().for_each(|(node_id, queue)| {
            msgs_to_relay.push((*node_id, queue.pop()));
        });
        msgs_to_relay
    }

    pub fn data_count_in_queue(&self) -> Vec<usize> {
        self.queues
            .iter()
            .map(|(_, queue)| queue.data_count())
            .collect()
    }

    fn check_and_update_cache(&mut self, msg: MessageId, sending: bool) -> bool {
        if let Some(received_msgs) = &mut self.received_msgs {
            let first_received = if let Some(count) = received_msgs.get_mut(&msg) {
                *count += 1;
                false
            } else {
                received_msgs.insert(msg, if sending { 0 } else { 1 });
                true
            };

            // If the message have been received from all connected peers, remove it from the cache
            // because there is no possibility that the message will be received again.
            if received_msgs.get(&msg).unwrap() == &self.peering_degree {
                tracing::debug!("Remove message from cache: {}", msg);
                received_msgs.remove(&msg);
            }

            first_received
        } else {
            true
        }
    }
}
