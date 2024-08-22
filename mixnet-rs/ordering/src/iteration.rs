use std::path::Path;

use protocol::{
    node::{MessageId, Node, NodeId},
    queue::{Message, QueueConfig, QueueType},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{ordercoeff::Sequence, paramset::ParamSet};

const RECEIVER_ID: NodeId = NodeId::MAX;

pub fn run_iteration(
    paramset: ParamSet,
    seed: u64,
    out_latency_path: &str,
    out_sent_sequence_path: &str,
    out_received_sequence_path: &str,
    out_data_msg_counts_path: &str,
    out_ordering_coeff_path: &str,
) {
    if paramset.random_topology {
        run_iteration_with_random_topology(
            paramset,
            seed,
            out_latency_path,
            out_sent_sequence_path,
            out_received_sequence_path,
            out_data_msg_counts_path,
            out_ordering_coeff_path,
        )
    } else {
        run_iteration_without_random_topology(
            paramset,
            seed,
            out_latency_path,
            out_sent_sequence_path,
            out_received_sequence_path,
            out_data_msg_counts_path,
            out_ordering_coeff_path,
        )
    }
}

fn run_iteration_without_random_topology(
    paramset: ParamSet,
    seed: u64,
    out_latency_path: &str,
    out_sent_sequence_path: &str,
    out_received_sequence_path_prefix: &str,
    out_queue_data_msg_counts_path: &str,
    out_ordering_coeff_path: &str,
) {
    assert!(!paramset.random_topology);

    // Ensure that all output files do not exist
    for path in &[
        out_latency_path,
        out_sent_sequence_path,
        out_received_sequence_path_prefix,
        out_queue_data_msg_counts_path,
    ] {
        assert!(!Path::new(path).exists(), "File already exists: {path}");
    }

    // Initialize mix nodes
    let mut next_node_id: NodeId = 0;
    let mut mixnodes: FxHashMap<NodeId, Node> = FxHashMap::default();
    let mut paths: Vec<Vec<NodeId>> = Vec::with_capacity(paramset.num_paths as usize);
    for _ in 0..paramset.num_paths {
        let mut ids = Vec::with_capacity(paramset.num_mixes as usize);
        for _ in 0..paramset.num_mixes {
            let id = next_node_id;
            next_node_id += 1;
            mixnodes.insert(
                id,
                Node::new(
                    QueueConfig {
                        queue_type: paramset.queue_type,
                        seed,
                        min_queue_size: paramset.min_queue_size,
                    },
                    paramset.peering_degree,
                    paramset.random_topology, // disable cache
                ),
            );
            ids.push(id);
        }
        paths.push(ids);
    }

    // Connect mix nodes
    for path in paths.iter() {
        for (i, id) in path.iter().enumerate() {
            if i != path.len() - 1 {
                let peer_id = path[i + 1];
                mixnodes.get_mut(id).unwrap().connect(peer_id);
            } else {
                mixnodes.get_mut(id).unwrap().connect(RECEIVER_ID);
            }
        }
    }
    let sender_peers: Vec<NodeId> = paths.iter().map(|path| path[0]).collect();

    let mut next_msg_id: MessageId = 0;

    // Virtual discrete time
    let mut vtime: f32 = 0.0;
    // Transmission interval that each queue must release a message
    let transmission_interval = 1.0 / paramset.transmission_rate as f32;
    // Results
    let mut all_sent_count = 0; // all data + noise sent by the sender
    let mut sent_times: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut latencies: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut sent_sequence = Sequence::new();
    let mut received_sequences: FxHashMap<NodeId, Sequence> = FxHashMap::default();
    let mut queue_data_msg_counts: Vec<FxHashMap<NodeId, Vec<usize>>> = Vec::new();

    let mut data_msg_rng = StdRng::seed_from_u64(seed);
    loop {
        tracing::trace!(
            "VTIME:{}, ALL_SENT:{}, DATA_SENT:{}, DATA_RECEIVED:{}",
            vtime,
            all_sent_count,
            sent_times.len(),
            latencies.len()
        );

        // The sender emits a message (data or noise) to all adjacent peers.
        if all_sent_count < paramset.num_sender_msgs as usize {
            if try_probability(&mut data_msg_rng, paramset.sender_data_msg_prob) {
                let msg = next_msg_id;
                next_msg_id += 1;
                sender_peers.iter().for_each(|peer_id| {
                    mixnodes.get_mut(peer_id).unwrap().receive(msg, None);
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

        // Each mix node add a new data message to its queue with a certain probability
        if try_probability(&mut data_msg_rng, paramset.mix_data_msg_prob) {
            for (_, node) in mixnodes.iter_mut() {
                node.send(next_msg_id);
                next_msg_id += 1;
                // Don't put the msg into the sent_sequence
                // because sent_sequence is only for recording messages sent by the sender, not the mixnode.
            }
        }

        // Each mix node relays a message (data or noise) to the next mix node or the receiver.
        // As the receiver, record the time and order of the received messages.
        //
        // source -> (destination, msg)
        let mut all_msgs_to_relay: Vec<(NodeId, Vec<(NodeId, Message<MessageId>)>)> = Vec::new();
        for (node_id, node) in mixnodes.iter_mut() {
            all_msgs_to_relay.push((*node_id, node.read_queues()));
        }
        all_msgs_to_relay
            .into_iter()
            .for_each(|(sender_id, msgs_to_relay)| {
                msgs_to_relay.into_iter().for_each(|(peer_id, msg)| {
                    if peer_id == RECEIVER_ID {
                        match msg {
                            Message::Data(msg) => {
                                latencies
                                    .entry(msg)
                                    .or_insert(vtime - sent_times.get(&msg).unwrap());
                                received_sequences
                                    .entry(sender_id)
                                    .or_insert(Sequence::new())
                                    .add_message(msg);
                            }
                            Message::Noise => {
                                received_sequences
                                    .entry(sender_id)
                                    .or_insert(Sequence::new())
                                    .add_noise();
                            }
                        }
                    } else if let Message::Data(msg) = msg {
                        mixnodes
                            .get_mut(&peer_id)
                            .unwrap()
                            .receive(msg, Some(sender_id));
                    }
                });
            });

        // Record the number of data messages in each mix node's queues
        let mut counts: FxHashMap<NodeId, Vec<usize>> = FxHashMap::default();
        mixnodes.iter().for_each(|(id, node)| {
            counts.insert(*id, node.queue_data_msg_counts());
        });
        queue_data_msg_counts.push(counts);

        // If all data amessages (that the sender has to send) have been received by the receiver,
        // stop the iteration.
        if all_sent_count == paramset.num_sender_msgs as usize
            && sent_times.len() == latencies.len()
        {
            break;
        }

        vtime += transmission_interval;
    }

    // Save results to CSV files
    save_latencies(&latencies, &sent_times, out_latency_path);
    save_sequence(&sent_sequence, out_sent_sequence_path);
    save_sequences(&received_sequences, out_received_sequence_path_prefix);
    save_queue_data_msg_counts(
        &queue_data_msg_counts,
        transmission_interval,
        out_queue_data_msg_counts_path,
    );
    // Calculate ordering coefficients and save them to a CSV file.
    if paramset.queue_type != QueueType::NonMix {
        let mut coeffs: Vec<[u64; 2]> = Vec::new();
        for (_, recv_seq) in received_sequences.iter() {
            let casual = sent_sequence.ordering_coefficient(recv_seq, true);
            let weak = sent_sequence.ordering_coefficient(recv_seq, false);
            coeffs.push([casual, weak]);
        }
        save_ordering_coefficients(&coeffs, out_ordering_coeff_path);
    }
}

fn run_iteration_with_random_topology(
    paramset: ParamSet,
    seed: u64,
    out_latency_path: &str,
    out_sent_sequence_path: &str,
    out_received_sequence_path: &str,
    out_data_msg_counts_path: &str,
    out_ordering_coeff_path: &str,
) {
    assert!(paramset.random_topology);
    todo!()
}

fn try_probability(rng: &mut StdRng, prob: f32) -> bool {
    assert!(
        (0.0..=1.0).contains(&prob),
        "Probability must be in [0, 1]."
    );
    rng.gen::<f32>() < prob
}

fn save_latencies(
    latencies: &FxHashMap<MessageId, f32>,
    sent_times: &FxHashMap<MessageId, f32>,
    path: &str,
) {
    let mut writer = csv::Writer::from_path(path).unwrap();
    writer
        .write_record(["latency", "sent_time", "received_time"])
        .unwrap();
    for (msg, latency) in latencies.iter() {
        let sent_time = sent_times.get(msg).unwrap();
        writer
            .write_record(&[
                latency.to_string(),
                sent_time.to_string(),
                (sent_time + latency).to_string(),
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

fn save_sequences(sequences: &FxHashMap<NodeId, Sequence>, path_prefix: &str) {
    sequences.iter().enumerate().for_each(|(i, (_, seq))| {
        save_sequence(seq, &format!("{path_prefix}_{i}.csv"));
    });
}

fn save_queue_data_msg_counts(data: &[FxHashMap<NodeId, Vec<usize>>], interval: f32, path: &str) {
    let mut writer = csv::Writer::from_path(path).unwrap();

    let mut header = vec!["vtime".to_string()];
    data[0].iter().for_each(|(node_id, counts)| {
        let num_queues = counts.len();
        (0..num_queues).for_each(|q_idx| {
            header.push(format!("node{node_id}_q{q_idx}"));
        });
    });
    writer.write_record(header).unwrap();

    data.iter().enumerate().for_each(|(i, counts_per_node)| {
        let mut row = vec![(i as f64 * interval as f64).to_string()];
        counts_per_node.iter().for_each(|(_, counts)| {
            row.extend(
                counts
                    .iter()
                    .map(|count| count.to_string())
                    .collect::<Vec<String>>(),
            );
        });
        writer.write_record(row).unwrap();
    });
    writer.flush().unwrap();
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
