use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use tracing::info;

use node::{MessageId, Node, NodeId};

mod node;

fn main() {
    tracing_subscriber::fmt::init();

    let seed: u64 = 0;
    let gtr: f32 = 10.0;
    let num_nodes: u16 = 10;
    let peering_degree: u16 = 2;
    let num_senders: u16 = 1;
    let num_sending_msgs: u16 = 3;
    assert!(num_nodes >= 2);
    assert!(num_senders <= num_nodes);

    // Initialize nodes
    let mut nodes: HashMap<NodeId, Node> = HashMap::new();
    for i in 0..num_nodes {
        nodes.insert(i, Node::new(i));
    }

    // Connect nodes
    let topology = build_topology(num_nodes, peering_degree, seed);
    for (node_id, peers) in topology.iter() {
        peers.iter().for_each(|peer_id| {
            nodes.get_mut(node_id).unwrap().connect(*peer_id);
        });
    }

    let sender_ids: Vec<NodeId> = (0..num_senders).collect();

    let mut vtime: f32 = 0.0;
    let interval: f32 = 1.0 / gtr;
    let mut next_msg_id: MessageId = 0;
    let mut sent_msgs: HashMap<MessageId, (f32, u16)> = HashMap::new();
    let mut disseminated_msgs: HashMap<MessageId, f32> = HashMap::new();
    loop {
        // Send new messages
        assert!(sent_msgs.len() % (num_senders as usize) == 0);
        if sent_msgs.len() / (num_senders as usize) < num_sending_msgs as usize {
            for sender_id in sender_ids.iter() {
                nodes.get_mut(sender_id).unwrap().send(next_msg_id);
                sent_msgs.insert(next_msg_id, (vtime, 1));
                next_msg_id += 1;
            }
        }

        // Collect messages to relay
        let mut all_msgs_to_relay = Vec::new();
        for (node_id, node) in nodes.iter_mut() {
            let msgs_to_relay = node.read_queues();
            msgs_to_relay.iter().for_each(|(receiver_id, msg)| {
                all_msgs_to_relay.push((*receiver_id, *msg, *node_id));
            });
        }

        // Relay the messages
        all_msgs_to_relay
            .into_iter()
            .for_each(|(receiver_id, msg, sender_id)| {
                if nodes.get_mut(&receiver_id).unwrap().receive(msg, sender_id) {
                    let (sent_time, num_received_nodes) = sent_msgs.get_mut(&msg).unwrap();
                    *num_received_nodes += 1;
                    if *num_received_nodes == num_nodes {
                        assert!(!disseminated_msgs.contains_key(&msg));
                        disseminated_msgs.insert(msg, vtime - *sent_time);
                    }
                }
            });

        // Check if all messages have been disseminated to all nodes.
        if disseminated_msgs.len() == (num_senders * num_sending_msgs) as usize {
            info!(
                "vtime:{vtime}: All {} messages have been disseminated to all nodes. Exiting...",
                disseminated_msgs.len()
            );
            for (msg, latency) in disseminated_msgs.iter() {
                info!(
                    "Message {} took {} time units to disseminate.",
                    msg, latency
                );
            }
            break;
        } else {
            info!(
                "vtime:{vtime}: {} messages have been disseminated to all nodes.",
                disseminated_msgs.len()
            );
        }

        vtime += interval;
    }
}

fn build_topology(
    num_nodes: u16,
    peering_degree: u16,
    seed: u64,
) -> HashMap<NodeId, HashSet<NodeId>> {
    let mut rng = StdRng::seed_from_u64(seed);

    loop {
        let mut topology: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        for node in 0..num_nodes {
            topology.insert(node, HashSet::new());
        }

        for node in 0..num_nodes {
            let mut others: Vec<NodeId> = Vec::new();
            for other in (0..node).chain(node + 1..num_nodes) {
                // Check if the other node is not already connected to the current node
                // and the other node has not reached the peering degree.
                if !topology.get(&node).unwrap().contains(&other)
                    && topology.get(&other).unwrap().len() < peering_degree as usize
                {
                    others.push(other);
                }
            }

            // How many more connections the current node needs
            let num_needs = peering_degree as usize - topology.get(&node).unwrap().len();
            // Smaple peers as many as possible and connect them to the current node
            let k = min(num_needs, others.len());
            others.as_mut_slice().shuffle(&mut rng);
            others.into_iter().take(k).for_each(|peer| {
                topology.get_mut(&node).unwrap().insert(peer);
                topology.get_mut(&peer).unwrap().insert(node);
            });
        }

        if are_all_nodes_connected(&topology) {
            return topology;
        }
    }
}

fn are_all_nodes_connected(topology: &HashMap<NodeId, HashSet<NodeId>>) -> bool {
    let start_node = topology.keys().next().unwrap();
    let visited = dfs(topology, *start_node);
    visited.len() == topology.len()
}

fn dfs(topology: &HashMap<NodeId, HashSet<NodeId>>, start_node: NodeId) -> HashSet<NodeId> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<NodeId> = Vec::new();
    stack.push(start_node);
    while let Some(node) = stack.pop() {
        visited.insert(node);
        for peer in topology.get(&node).unwrap().iter() {
            if !visited.contains(peer) {
                stack.push(*peer);
            }
        }
    }
    visited
}
