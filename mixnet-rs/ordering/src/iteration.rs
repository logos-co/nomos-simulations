use std::collections::hash_map::Entry;

use protocol::{
    node::{MessagesToRelay, Node, NodeId},
    queue::Message,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{
    message::{DataMessage, DataMessageGenerator},
    outputs::Outputs,
    paramset::ParamSet,
    topology::{build_random_network, build_striped_network, RECEIVER_NODE_ID},
};

pub fn run_iteration(paramset: ParamSet, seed: u64, outputs: &mut Outputs) -> f32 {
    let (mut mixnodes, all_sender_peers, receiver_peers) = if paramset.random_topology {
        build_random_network(&paramset, seed, outputs)
    } else {
        build_striped_network(&paramset, seed)
    };
    // Check node ID consistency
    for (i, node) in mixnodes.iter().enumerate() {
        assert_eq!(node.id as usize, i);
    }

    // For N senders + 1 mix (all mixnodes will share the same sender ID)
    let mut data_msg_gen = DataMessageGenerator::new(paramset.num_senders + 1);
    let mix_msg_sender_id = paramset.num_senders;

    // Virtual discrete time
    let mut vtime: f32 = 0.0;
    // Transmission interval that each queue must release a message
    let transmission_interval = 1.0 / paramset.transmission_rate as f32;
    // Results
    let mut all_sent_count = 0; // all data + noise sent by all senders
    let all_sent_count_target = (paramset.num_sender_msgs as usize)
        .checked_mul(paramset.num_senders as usize)
        .unwrap();
    let mut sent_data_msgs: FxHashMap<DataMessage, f32> = FxHashMap::default();
    let mut recv_data_msgs: FxHashMap<DataMessage, f32> = FxHashMap::default();

    outputs.write_header_queue_data_msg_counts(&mixnodes);

    let mut data_msg_rng = StdRng::seed_from_u64(seed);
    loop {
        tracing::trace!(
            "VTIME:{}, ALL_SENT:{}, DATA_SENT:{}, DATA_RECEIVED:{}",
            vtime,
            all_sent_count,
            sent_data_msgs.len(),
            recv_data_msgs.len(),
        );

        // All senders emit a message (data or noise) to all of their own adjacent peers.
        if all_sent_count < all_sent_count_target {
            // For each sender
            for (sender_idx, sender_peers) in all_sender_peers.iter() {
                if try_probability(&mut data_msg_rng, paramset.sender_data_msg_prob) {
                    let msg = data_msg_gen.next(sender_idx);
                    sender_peers.iter().for_each(|peer_id| {
                        mixnodes
                            .get_mut(*peer_id as usize)
                            .unwrap()
                            .receive(msg, None);
                    });
                    sent_data_msgs.insert(msg, vtime);
                    outputs.add_sent_msg(&msg)
                } else {
                    // Generate noise and add it to the sequence to calculate ordering coefficients later,
                    // but don't need to send it to the mix nodes
                    // because the mix nodes will anyway drop the noise,
                    // and we don't need to record what the mix nodes receive.
                    outputs.add_sent_noise(sender_idx);
                }
                all_sent_count += 1;
            }
        }

        // Each mix node add a new data message to its queue with a certain probability
        if paramset.mix_data_msg_prob > 0.0 {
            for node in mixnodes.iter_mut() {
                if try_probability(&mut data_msg_rng, paramset.mix_data_msg_prob) {
                    node.send(data_msg_gen.next(mix_msg_sender_id));
                    // We don't put the msg into the sent_sequence
                    // because sent_sequence is only for recording messages sent by the senders, not the mixnode.
                }
            }
        }

        // Each mix node relays a message (data or noise) to the next mix node or the receiver.
        // As the receiver, record the time and order of the received messages.
        AllMessagesToRelay::new(&mut mixnodes).into_iter().for_each(
            |(relayer_id, msgs_to_relay)| {
                msgs_to_relay.into_iter().for_each(|(peer_id, msg)| {
                    if peer_id == RECEIVER_NODE_ID {
                        match msg {
                            Message::Data(msg) => {
                                // If msg was sent by the sender (not by any mix)
                                if let Some(&sent_time) = sent_data_msgs.get(&msg) {
                                    // If this is the first time to see the msg,
                                    // update stats that must ignore duplicate messages.
                                    if let Entry::Vacant(e) = recv_data_msgs.entry(msg) {
                                        e.insert(vtime);
                                        outputs.add_latency(&msg, sent_time, vtime);
                                    }
                                }
                                // Record msg to the sequence
                                let conn_idx = receiver_peers.conn_idx(&relayer_id).unwrap();
                                outputs.add_recv_msg(&msg, conn_idx);
                            }
                            Message::Noise => {
                                // Record noise to the sequence
                                let conn_idx = receiver_peers.conn_idx(&relayer_id).unwrap();
                                outputs.add_recv_noise(conn_idx);
                            }
                        }
                    } else if let Message::Data(msg) = msg {
                        let peer = mixnodes.get_mut(peer_id as usize).unwrap();
                        assert_eq!(peer.id, peer_id);
                        peer.receive(msg, Some(relayer_id));
                    }
                });
            },
        );

        // Record the number of data messages in each mix node's queues
        outputs.add_queue_data_msg_counts(vtime, &mixnodes);

        // If all senders finally emitted all data+noise messages,
        // and If all data messages have been received by the receiver,
        // stop the iteration.
        if all_sent_count == all_sent_count_target && sent_data_msgs.len() == recv_data_msgs.len() {
            break;
        }

        vtime += transmission_interval;
    }

    vtime
}

fn try_probability(rng: &mut StdRng, prob: f32) -> bool {
    assert!(
        (0.0..=1.0).contains(&prob),
        "Probability must be in [0, 1]."
    );
    rng.gen::<f32>() < prob
}

struct AllMessagesToRelay(Vec<(NodeId, MessagesToRelay<DataMessage>)>);

impl AllMessagesToRelay {
    fn new(mixnodes: &mut [Node<DataMessage>]) -> Self {
        let mut all_msgs_to_relay = Vec::with_capacity(mixnodes.len());
        for node in mixnodes.iter_mut() {
            all_msgs_to_relay.push((node.id, node.read_queues()));
        }
        Self(all_msgs_to_relay)
    }

    fn into_iter(self) -> impl Iterator<Item = (NodeId, Vec<(NodeId, Message<DataMessage>)>)> {
        self.0.into_iter()
    }
}
