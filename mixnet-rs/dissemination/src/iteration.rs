use std::error::Error;

use rand::{rngs::StdRng, seq::SliceRandom, RngCore, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{
    node::{MessageId, Node, NodeId},
    paramset::ParamSet,
    queue::QueueConfig,
    topology::{build_topology, Topology},
};

// An interval that the sender nodes send (schedule) new messages
const MSG_INTERVAL: f32 = 1.0;

pub fn run_iteration(paramset: ParamSet, seed: u64, out_csv_path: &str, topology_path: &str) {
    // Initialize nodes (not connected with each other yet)
    let mut nodes: Vec<Node> = Vec::new();
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    let peering_degrees = paramset.gen_peering_degrees(seed);
    tracing::debug!("PeeringDegrees initialized.");

    for node_id in 0..paramset.num_nodes {
        nodes.push(Node::new(
            QueueConfig {
                queue_type: paramset.queue_type,
                seed: queue_seed_rng.next_u64(),
                min_queue_size: paramset.min_queue_size,
            },
            peering_degrees[node_id as usize],
        ));
    }
    tracing::debug!("Nodes initialized.");

    // Build a random topology, and connect nodes with each other
    let topology = build_topology(paramset.num_nodes, &peering_degrees, seed);
    tracing::debug!("Topology built.");
    save_topology(&topology, topology_path).unwrap();
    tracing::debug!("Topology saved.");
    for (node_id, peers) in topology.iter().enumerate() {
        peers.iter().for_each(|peer_id| {
            nodes[node_id].connect(*peer_id);
        });
    }
    tracing::debug!("Nodes connected");

    let mut sender_selector = SenderSelector::new(
        (0..paramset.num_nodes).collect(),
        paramset.num_senders,
        paramset.random_senders_every_time,
        seed,
    );

    // To generate unique message IDs
    let mut next_msg_id: MessageId = 0;
    let total_num_msgs = paramset.total_num_messages();
    // msg_id -> (sent_vtime, num_received_nodes)
    let mut message_tracker: FxHashMap<MessageId, (f32, u16)> = FxHashMap::default();
    // To keep track of how many messages have been disseminated to all nodes
    let mut num_disseminated_msgs = 0;

    let mut writer = csv::Writer::from_path(out_csv_path).unwrap();
    writer
        .write_record(["dissemination_time", "sent_time", "all_received_time"])
        .unwrap();
    writer.flush().unwrap();

    // Virtual discrete time
    let mut vtime: f32;
    // Transmission interval that each queue must release a message
    let transmission_interval = 1.0 / paramset.transmission_rate as f32;
    // Jump `vtime` to one of the following two vtimes.
    // 1. The next time to send (schedule) a message. Increased by `MSG_INTERVAL`.
    let mut next_messaging_vtime: f32 = 0.0;
    // 2. The next time to release a message from each queue and relay them. Increased by `transmission_interval`.
    let mut next_transmission_vtime: f32 = 0.0;
    loop {
        // If there are still messages to be sent (scheduled),
        // and if the next time to send a message is earlier than the next time to relay messages.
        if next_msg_id < total_num_msgs && next_messaging_vtime <= next_transmission_vtime {
            // Send new messages
            vtime = next_messaging_vtime;
            next_messaging_vtime += MSG_INTERVAL;

            send_messages(
                vtime,
                sender_selector.next(),
                &mut nodes,
                &mut next_msg_id,
                &mut message_tracker,
            );
        } else {
            // Release a message from each queue and relay all of them
            vtime = next_transmission_vtime;
            next_transmission_vtime += transmission_interval;

            relay_messages(
                vtime,
                &mut nodes,
                &mut message_tracker,
                &mut num_disseminated_msgs,
                &mut writer,
            );

            // Check if all messages have been disseminated to all nodes.
            if num_disseminated_msgs == total_num_msgs as usize {
                break;
            }
        }
    }
}

fn send_messages(
    vtime: f32,
    sender_ids: &[NodeId],
    nodes: &mut [Node],
    next_msg_id: &mut MessageId,
    message_tracker: &mut FxHashMap<MessageId, (f32, u16)>,
) {
    for &sender_id in sender_ids.iter() {
        nodes[sender_id as usize].send(*next_msg_id);
        message_tracker.insert(*next_msg_id, (vtime, 1));
        *next_msg_id += 1;
    }
}

fn relay_messages(
    vtime: f32,
    nodes: &mut [Node],
    message_tracker: &mut FxHashMap<MessageId, (f32, u16)>,
    num_disseminated_msgs: &mut usize,
    writer: &mut csv::Writer<std::fs::File>,
) {
    // Collect messages to relay
    let mut all_msgs_to_relay: Vec<Vec<(NodeId, MessageId)>> = Vec::new();
    for node in nodes.iter_mut() {
        all_msgs_to_relay.push(node.read_queues());
    }

    // Relay the messages
    all_msgs_to_relay
        .into_iter()
        .enumerate()
        .for_each(|(sender_id, msgs_to_relay)| {
            let sender_id: NodeId = sender_id.try_into().unwrap();
            msgs_to_relay.into_iter().for_each(|(receiver_id, msg)| {
                if nodes[receiver_id as usize].receive(msg, sender_id) {
                    let (sent_time, num_received_nodes) = message_tracker.get_mut(&msg).unwrap();
                    *num_received_nodes += 1;
                    if *num_received_nodes as usize == nodes.len() {
                        let dissemination_time = vtime - *sent_time;
                        writer
                            .write_record(&[
                                dissemination_time.to_string(),
                                sent_time.to_string(),
                                vtime.to_string(),
                            ])
                            .unwrap();
                        writer.flush().unwrap();
                        *num_disseminated_msgs += 1;

                        message_tracker.remove(&msg);
                    }
                }
            })
        });
}

fn save_topology(topology: &Topology, topology_path: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path(topology_path)?;
    wtr.write_record(["node", "num_peers", "peers"])?;

    for (node, peers) in topology.iter().enumerate() {
        let node: NodeId = node.try_into().unwrap();
        let peers_str: Vec<String> = peers.iter().map(|peer_id| peer_id.to_string()).collect();
        wtr.write_record(&[
            node.to_string(),
            peers.len().to_string(),
            format!("[{}]", peers_str.join(",")),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

struct SenderSelector {
    candidates: Vec<NodeId>,
    num_senders: NodeId,
    random_senders_every_time: bool,
    rng: StdRng,
}

impl SenderSelector {
    fn new(
        candidates: Vec<NodeId>,
        num_senders: u32,
        random_senders_every_time: bool,
        seed: u64,
    ) -> Self {
        assert!(candidates.len() >= num_senders as usize);
        Self {
            candidates,
            num_senders,
            random_senders_every_time,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn next(&mut self) -> &[NodeId] {
        if !self.random_senders_every_time {
            // It's okay to pick the first `num_senders` nodes
            // because the topology is randomly generated.
            &self.candidates[..self.num_senders as usize]
        } else {
            self.candidates.shuffle(&mut self.rng);
            &self.candidates[..self.num_senders as usize]
        }
    }
}
