use std::{collections::hash_map::Entry, fs::File, path::Path};

use csv::Writer;
use protocol::{
    node::{MessageId, Node, NodeId},
    queue::{Message, QueueConfig, QueueType},
    topology::{build_topology, save_topology},
};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, RngCore, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{ordercoeff::Sequence, paramset::ParamSet};

const RECEIVER_ID: NodeId = NodeId::MAX;

pub fn run_iteration(
    paramset: ParamSet,
    seed: u64,
    out_latency_path: &str,
    out_sent_sequence_path: &str,
    out_received_sequence_path_prefix: &str,
    out_queue_data_msg_counts_path: &str,
    out_ordering_coeff_path: Option<String>,
    out_topology_path: &str,
) -> f32 {
    // Ensure that all output files do not exist
    for path in &[
        out_latency_path,
        out_sent_sequence_path,
        out_received_sequence_path_prefix,
        out_queue_data_msg_counts_path,
        out_topology_path,
    ] {
        assert!(!Path::new(path).exists(), "File already exists: {path}");
    }
    if let Some(path) = &out_ordering_coeff_path {
        assert!(!Path::new(path).exists(), "File already exists: {path}");
    }

    let (mut mixnodes, sender_peers_list) = if paramset.random_topology {
        build_random_network(&paramset, seed, out_topology_path)
    } else {
        build_striped_network(&paramset, seed)
    };

    let mut next_msg_id: MessageId = 0;

    // Virtual discrete time
    let mut vtime: f32 = 0.0;
    // Transmission interval that each queue must release a message
    let transmission_interval = 1.0 / paramset.transmission_rate as f32;
    // Results
    let mut all_sent_count = 0; // all data + noise sent by all senders
    let target_all_sent_count = (paramset.num_sender_msgs as usize)
        .checked_mul(paramset.num_senders as usize)
        .unwrap();
    let mut sent_times: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut recv_times: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut latencies: Vec<(MessageId, f32)> = Vec::new();
    let mut sent_sequence = Sequence::new();
    let mut received_sequences: FxHashMap<NodeId, Sequence> = FxHashMap::default();
    let mut unified_received_sequence = if paramset.random_topology {
        Some(Sequence::new())
    } else {
        None
    };
    let mut queue_data_msg_counts_writer =
        new_queue_data_msg_counts_writer(out_queue_data_msg_counts_path, &mixnodes);

    let mut data_msg_rng = StdRng::seed_from_u64(seed);
    loop {
        tracing::trace!(
            "VTIME:{}, ALL_SENT:{}, DATA_SENT:{}, DATA_RECEIVED:{}",
            vtime,
            all_sent_count,
            sent_times.len(),
            latencies.len()
        );

        // All senders emit a message (data or noise) to all of their own adjacent peers.
        if all_sent_count < target_all_sent_count {
            for sender_peers in sender_peers_list.iter() {
                if try_probability(&mut data_msg_rng, paramset.sender_data_msg_prob) {
                    let msg = next_msg_id;
                    next_msg_id += 1;
                    sender_peers.iter().for_each(|peer_id| {
                        mixnodes
                            .get_mut(*peer_id as usize)
                            .unwrap()
                            .receive(msg, None);
                    });
                    sent_times.insert(msg, vtime);
                    sent_sequence.add_message(msg);
                } else {
                    // Generate noise and add it to the sequence to calculate ordering coefficients later,
                    // but don't need to send it to the mix nodes
                    // because the mix nodes will anyway drop the noise,
                    // and we don't need to record what the mix nodes receive.
                    sent_sequence.add_noise();
                }
                all_sent_count += 1;
            }
        }

        // Each mix node add a new data message to its queue with a certain probability
        for node in mixnodes.iter_mut() {
            if try_probability(&mut data_msg_rng, paramset.mix_data_msg_prob) {
                node.send(next_msg_id);
                next_msg_id += 1;
                // Don't put the msg into the sent_sequence
                // because sent_sequence is only for recording messages sent by the senders, not the mixnode.
            }
        }

        // Each mix node relays a message (data or noise) to the next mix node or the receiver.
        // As the receiver, record the time and order of the received messages.
        //
        // source -> (destination, msg)
        let mut all_msgs_to_relay: Vec<(NodeId, Vec<(NodeId, Message<MessageId>)>)> = Vec::new();
        for (node_id, node) in mixnodes.iter_mut().enumerate() {
            all_msgs_to_relay.push((node_id.try_into().unwrap(), node.read_queues()));
        }
        all_msgs_to_relay
            .into_iter()
            .for_each(|(mix_id, msgs_to_relay)| {
                msgs_to_relay.into_iter().for_each(|(peer_id, msg)| {
                    if peer_id == RECEIVER_ID {
                        match msg {
                            Message::Data(msg) => {
                                // If msg was sent by the sender (not by any mix)
                                if let Some(&sent_time) = sent_times.get(&msg) {
                                    // If this is the first time to see the msg
                                    if let Entry::Vacant(e) = recv_times.entry(msg) {
                                        e.insert(vtime);
                                        latencies.push((msg, vtime - sent_time));
                                        if let Some(unified_recv_seq) =
                                            &mut unified_received_sequence
                                        {
                                            unified_recv_seq.add_message(msg);
                                        }
                                    }
                                    received_sequences
                                        .entry(mix_id)
                                        .or_insert(Sequence::new())
                                        .add_message(msg);
                                }
                            }
                            Message::Noise => {
                                received_sequences
                                    .entry(mix_id)
                                    .or_insert(Sequence::new())
                                    .add_noise();
                            }
                        }
                    } else if let Message::Data(msg) = msg {
                        mixnodes
                            .get_mut(peer_id as usize)
                            .unwrap()
                            .receive(msg, Some(mix_id));
                    }
                });
            });

        // Record the number of data messages in each mix node's queues
        append_queue_data_msg_counts(&mixnodes, vtime, &mut queue_data_msg_counts_writer);

        // If all data messages (that have been sent by the senders) have been received by the receiver,
        // stop the iteration.
        if all_sent_count == target_all_sent_count && sent_times.len() == latencies.len() {
            break;
        }

        vtime += transmission_interval;
    }

    // Save results to CSV files
    save_latencies(&latencies, &sent_times, &recv_times, out_latency_path);
    save_sequence(&sent_sequence, out_sent_sequence_path);
    // Sort received_sequences
    let mut node_ids: Vec<NodeId> = received_sequences.keys().cloned().collect();
    node_ids.sort();
    let received_sequences: Vec<Sequence> = node_ids
        .iter()
        .map(|node_id| received_sequences.remove(node_id).unwrap())
        .collect();
    save_sequences(&received_sequences, out_received_sequence_path_prefix);
    if let Some(unified_recv_seq) = &unified_received_sequence {
        save_sequence(
            unified_recv_seq,
            format!("{out_received_sequence_path_prefix}_unified.csv").as_str(),
        );
    }
    // Calculate ordering coefficients and save them to a CSV file (if enabled)
    if let Some(out_ordering_coeff_path) = &out_ordering_coeff_path {
        if paramset.queue_type != QueueType::NonMix {
            if let Some(unified_recv_seq) = &unified_received_sequence {
                let casual = sent_sequence.ordering_coefficient(unified_recv_seq, true);
                let weak = sent_sequence.ordering_coefficient(unified_recv_seq, false);
                save_ordering_coefficients(&[[casual, weak]], out_ordering_coeff_path);
            } else {
                let mut coeffs: Vec<[u64; 2]> = Vec::new();
                for recv_seq in received_sequences.iter() {
                    let casual = sent_sequence.ordering_coefficient(recv_seq, true);
                    let weak = sent_sequence.ordering_coefficient(recv_seq, false);
                    coeffs.push([casual, weak]);
                }
                save_ordering_coefficients(&coeffs, out_ordering_coeff_path);
            }
        }
    }

    vtime
}

fn build_striped_network(paramset: &ParamSet, seed: u64) -> (Vec<Node>, Vec<Vec<NodeId>>) {
    assert!(!paramset.random_topology);
    let mut next_node_id: NodeId = 0;
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    let mut mixnodes: Vec<Node> =
        Vec::with_capacity(paramset.num_paths as usize * paramset.num_mixes as usize);
    let mut paths: Vec<Vec<NodeId>> = Vec::with_capacity(paramset.num_paths as usize);
    for _ in 0..paramset.num_paths {
        let mut ids = Vec::with_capacity(paramset.num_mixes as usize);
        for _ in 0..paramset.num_mixes {
            let id = next_node_id;
            next_node_id += 1;
            mixnodes.push(Node::new(
                QueueConfig {
                    queue_type: paramset.queue_type,
                    seed: queue_seed_rng.next_u64(),
                    min_queue_size: paramset.min_queue_size,
                },
                paramset.peering_degree,
                false, // disable cache
            ));
            ids.push(id);
        }
        paths.push(ids);
    }

    // Connect mix nodes
    for path in paths.iter() {
        for (i, id) in path.iter().enumerate() {
            if i != path.len() - 1 {
                let peer_id = path[i + 1];
                mixnodes.get_mut(*id as usize).unwrap().connect(peer_id);
            } else {
                mixnodes.get_mut(*id as usize).unwrap().connect(RECEIVER_ID);
            }
        }
    }
    let sender_peers_list: Vec<Vec<NodeId>> =
        vec![
            paths.iter().map(|path| *path.first().unwrap()).collect();
            paramset.num_senders as usize
        ];
    (mixnodes, sender_peers_list)
}

fn build_random_network(
    paramset: &ParamSet,
    seed: u64,
    out_topology_path: &str,
) -> (Vec<Node>, Vec<Vec<NodeId>>) {
    assert!(paramset.random_topology);
    // Init mix nodes
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    let mut mixnodes: Vec<Node> = Vec::with_capacity(paramset.num_mixes as usize);
    for _ in 0..paramset.num_mixes {
        mixnodes.push(Node::new(
            QueueConfig {
                queue_type: paramset.queue_type,
                seed: queue_seed_rng.next_u64(),
                min_queue_size: paramset.min_queue_size,
            },
            paramset.peering_degree,
            true, // enable cache
        ));
    }

    // Choose sender's peers and receiver's peers randomly
    let mut peers_rng = StdRng::seed_from_u64(seed);
    let mut candidates: Vec<NodeId> = (0..paramset.num_mixes).collect();
    assert!(candidates.len() >= paramset.peering_degree as usize);
    let mut sender_peers_list: Vec<Vec<NodeId>> = Vec::with_capacity(paramset.num_senders as usize);
    for _ in 0..paramset.num_senders {
        candidates.as_mut_slice().shuffle(&mut peers_rng);
        sender_peers_list.push(
            candidates
                .iter()
                .cloned()
                .take(paramset.peering_degree as usize)
                .collect(),
        );
    }
    candidates.as_mut_slice().shuffle(&mut peers_rng);
    let receiver_peers: Vec<NodeId> = candidates
        .iter()
        .cloned()
        .take(paramset.peering_degree as usize)
        .collect();

    // Connect mix nodes
    let topology = build_topology(
        paramset.num_mixes,
        &vec![paramset.peering_degree; paramset.num_mixes as usize],
        seed,
    );
    save_topology(&topology, out_topology_path).unwrap();
    for (node_id, peers) in topology.iter().enumerate() {
        peers.iter().for_each(|peer_id| {
            mixnodes.get_mut(node_id).unwrap().connect(*peer_id);
        });
    }

    // Connect the selected mix nodes with the receiver
    for id in receiver_peers.iter() {
        mixnodes.get_mut(*id as usize).unwrap().connect(RECEIVER_ID);
    }

    (mixnodes, sender_peers_list)
}

fn try_probability(rng: &mut StdRng, prob: f32) -> bool {
    assert!(
        (0.0..=1.0).contains(&prob),
        "Probability must be in [0, 1]."
    );
    rng.gen::<f32>() < prob
}

fn save_latencies(
    latencies: &[(MessageId, f32)],
    sent_times: &FxHashMap<MessageId, f32>,
    recv_times: &FxHashMap<MessageId, f32>,
    path: &str,
) {
    let mut writer = csv::Writer::from_path(path).unwrap();
    writer
        .write_record(["msg_id", "latency", "sent_time", "received_time"])
        .unwrap();
    for (msg, latency) in latencies.iter() {
        let sent_time = sent_times.get(msg).unwrap();
        let recv_time = recv_times.get(msg).unwrap();
        writer
            .write_record(&[
                msg.to_string(),
                latency.to_string(),
                sent_time.to_string(),
                recv_time.to_string(),
            ])
            .unwrap();
    }
    writer.flush().unwrap();
}

fn save_sequence(seq: &Sequence, path: &str) {
    let mut writer = csv::Writer::from_path(path).unwrap();
    seq.iter().for_each(|entry| {
        writer.write_record([entry.to_string()]).unwrap();
    });
    writer.flush().unwrap();
}

fn save_sequences(sequences: &[Sequence], path_prefix: &str) {
    sequences.iter().enumerate().for_each(|(i, seq)| {
        save_sequence(seq, &format!("{path_prefix}_{i}.csv"));
    });
}

fn new_queue_data_msg_counts_writer(path: &str, mixnodes: &[Node]) -> Writer<File> {
    let mut writer = csv::Writer::from_path(path).unwrap();
    let mut header = vec!["vtime".to_string()];
    mixnodes
        .iter()
        .map(|node| node.queue_data_msg_counts())
        .enumerate()
        .for_each(|(node_id, counts)| {
            let num_queues = counts.len();
            (0..num_queues).for_each(|q_idx| {
                header.push(format!("node{node_id}_q{q_idx}"));
            });
        });
    writer.write_record(header).unwrap();
    writer.flush().unwrap();
    writer
}

fn append_queue_data_msg_counts(mixnodes: &[Node], vtime: f32, writer: &mut Writer<File>) {
    let mut row = vec![vtime.to_string()];
    mixnodes
        .iter()
        .map(|node| node.queue_data_msg_counts())
        .for_each(|counts| {
            row.extend(
                counts
                    .iter()
                    .map(|count| count.to_string())
                    .collect::<Vec<_>>(),
            );
        });
    writer.write_record(row).unwrap();
}

fn save_ordering_coefficients(data: &[[u64; 2]], path: &str) {
    let mut writer = csv::Writer::from_path(path).unwrap();
    writer.write_record(["path", "casual", "weak"]).unwrap();
    for (path_idx, [casual, weak]) in data.iter().enumerate() {
        writer
            .write_record([path_idx.to_string(), casual.to_string(), weak.to_string()])
            .unwrap();
    }
    writer.flush().unwrap();
}
