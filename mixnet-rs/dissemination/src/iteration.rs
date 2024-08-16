use std::{collections::HashMap, error::Error};

use rand::{rngs::StdRng, RngCore, SeedableRng};

use crate::{
    node::{MessageId, Node, NodeId},
    paramset::ParamSet,
    queue::QueueConfig,
    topology::{build_topology, Topology},
};

pub fn run_iteration(paramset: ParamSet, seed: u64, out_csv_path: &str, topology_path: &str) {
    // Initialize nodes
    let mut nodes: Vec<Node> = Vec::new();
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    for _ in 0..paramset.num_nodes {
        nodes.push(Node::new(QueueConfig {
            queue_type: paramset.queue_type,
            seed: queue_seed_rng.next_u64(),
            min_queue_size: paramset.min_queue_size,
        }));
    }

    // Connect nodes
    let topology = build_topology(paramset.num_nodes, paramset.peering_degree, seed);
    save_topology(&topology, topology_path).unwrap();
    for (node_id, peers) in topology.iter().enumerate() {
        peers.iter().for_each(|peer_id| {
            nodes[node_id].connect(*peer_id);
        });
    }

    let sender_ids: Vec<NodeId> = (0..paramset.num_senders).collect();

    // Virtual discrete time
    let mut vtime: f32 = 0.0;
    // Increase vtime according to the transmission rate
    let interval: f32 = 1.0 / paramset.transmission_rate as f32;
    // To generate unique message IDs
    let mut next_msg_id: MessageId = 0;
    // To keep track of when each message was sent and how many nodes received it
    let mut sent_msgs: HashMap<MessageId, (f32, u16)> = HashMap::new();
    // To keep track of how many messages have been disseminated to all nodes
    let mut num_disseminated_msgs = 0;

    let mut writer = csv::Writer::from_path(out_csv_path).unwrap();
    writer
        .write_record(["dissemination_time", "sent_time", "all_received_time"])
        .unwrap();

    loop {
        // Send new messages
        assert!(sent_msgs.len() % (paramset.num_senders as usize) == 0);
        if sent_msgs.len() / (paramset.num_senders as usize) < paramset.num_sent_msgs as usize {
            for &sender_id in sender_ids.iter() {
                nodes[sender_id as usize].send(next_msg_id);
                sent_msgs.insert(next_msg_id, (vtime, 1));
                next_msg_id += 1;
            }
        }

        // Collect messages to relay
        let mut all_msgs_to_relay = Vec::new();
        for (node_id, node) in nodes.iter_mut().enumerate() {
            let msgs_to_relay = node.read_queues();
            msgs_to_relay.iter().for_each(|(receiver_id, msg)| {
                all_msgs_to_relay.push((*receiver_id, *msg, node_id as u16));
            });
        }

        // Relay the messages
        all_msgs_to_relay
            .into_iter()
            .for_each(|(receiver_id, msg, sender_id)| {
                if nodes[receiver_id as usize].receive(msg, sender_id) {
                    let (sent_time, num_received_nodes) = sent_msgs.get_mut(&msg).unwrap();
                    *num_received_nodes += 1;
                    if *num_received_nodes == paramset.num_nodes {
                        let dissemination_time = vtime - *sent_time;
                        writer
                            .write_record(&[
                                dissemination_time.to_string(),
                                sent_time.to_string(),
                                vtime.to_string(),
                            ])
                            .unwrap();
                        num_disseminated_msgs += 1;
                    }
                }
            });

        // Check if all messages have been disseminated to all nodes.
        if num_disseminated_msgs == (paramset.num_senders * paramset.num_sent_msgs) as usize {
            break;
        }

        vtime += interval;
    }
}

fn save_topology(topology: &Topology, topology_path: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path(topology_path)?;
    wtr.write_record(["node", "num_peers", "peers"])?;

    for (node, peers) in topology.iter().enumerate() {
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
