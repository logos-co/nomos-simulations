use std::{fmt::Display, path::Path};

use queue::QueueConfig;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{
    node::{MessageId, Node},
    paramset::ParamSet,
};

pub fn run_iteration(
    paramset: ParamSet,
    seed: u64,
    out_latency_path: &str,
    out_sent_sequence_path: &str,
    out_received_sequence_path: &str,
    out_data_msg_counts_path: &str,
) {
    // Ensure that all output files do not exist
    for path in &[
        out_latency_path,
        out_sent_sequence_path,
        out_received_sequence_path,
        out_data_msg_counts_path,
    ] {
        assert!(!Path::new(path).exists(), "File already exists: {path}");
    }

    // Initialize a mix node
    let mut mixnode = Node::new(&QueueConfig {
        queue_type: paramset.queue_type,
        seed,
        min_queue_size: paramset.min_queue_size,
    });

    let mut next_msg_id: MessageId = 0;

    // Virtual discrete time
    let mut vtime: f32 = 0.0;
    // Transmission interval that each queue must release a message
    let transmission_interval = 1.0 / paramset.transmission_rate as f32;
    // Results
    let mut sent_times: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut latencies: FxHashMap<MessageId, f32> = FxHashMap::default();
    let mut sent_sequence = MessageSequence::new();
    let mut received_sequence = MessageSequence::new();
    let mut data_msg_counts_in_queue: Vec<u32> = Vec::new();

    let mut rng = StdRng::seed_from_u64(seed);
    loop {
        // The sender emits a message (data or noise) to the mix node.
        if sent_times.len() < paramset.num_sender_data_msgs as usize
            && try_probability(&mut rng, paramset.sender_data_msg_prob)
        {
            let msg = next_msg_id;
            next_msg_id += 1;
            mixnode.receive(msg);
            sent_times.insert(msg, vtime);
            sent_sequence.add_message(msg);
        } else {
            // Generate noise and add it to the sequence to calculate ordering coefficients later,
            // but don't need to send it to the mix node
            // because the mix node will anyway drop the noise,
            // and we don't need to record what the mix node receives.
            sent_sequence.add_noise();
        }

        // The mix node add a new data message to its queue with a certain probability
        if try_probability(&mut rng, paramset.mix_data_msg_prob) {
            mixnode.send(next_msg_id);
            next_msg_id += 1;
            // Don't put the msg into the sent_sequence
            // because sent_sequence is only for recording messages sent by the sender, not the mixnode.
        }

        // The mix node emits a message (data or noise) to the receiver.
        // As the receiver, record the time and order of the received messages.
        match mixnode.read_queue() {
            Some(msg) => {
                latencies.insert(msg, vtime - sent_times.get(&msg).unwrap());
                received_sequence.add_message(msg);
            }
            None => {
                received_sequence.add_noise();
            }
        }

        // Record the number of data messages in the mix node's queue
        data_msg_counts_in_queue.push(mixnode.message_count_in_queue());

        // If all messages have been received by the receiver, stop the iteration.
        assert!(latencies.len() <= paramset.num_sender_data_msgs as usize);
        if latencies.len() == paramset.num_sender_data_msgs as usize {
            break;
        }

        vtime += transmission_interval;
    }

    // Save results to CSV files
    save_latencies(&latencies, &sent_times, out_latency_path);
    save_sequence(&sent_sequence, out_sent_sequence_path);
    save_sequence(&received_sequence, out_received_sequence_path);
    save_data_msg_counts(
        &data_msg_counts_in_queue,
        transmission_interval,
        out_data_msg_counts_path,
    );
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

fn save_sequence(sequence: &MessageSequence, path: &str) {
    let mut writer = csv::Writer::from_path(path).unwrap();
    sequence.messages.iter().for_each(|entry| {
        writer.write_record([entry.to_string()]).unwrap();
    });
    writer.flush().unwrap();
}

fn save_data_msg_counts(
    data_msg_counts_in_queue: &[u32],
    interval: f32,
    out_data_msg_counts_path: &str,
) {
    let mut writer = csv::Writer::from_path(out_data_msg_counts_path).unwrap();
    writer
        .write_record(["vtime", "data_msg_count_in_queue"])
        .unwrap();
    data_msg_counts_in_queue
        .iter()
        .enumerate()
        .for_each(|(i, count)| {
            writer
                .write_record([(i as f64 * interval as f64).to_string(), count.to_string()])
                .unwrap();
        });
    writer.flush().unwrap();
}

struct MessageSequence {
    messages: Vec<SequenceEntry>,
}

impl MessageSequence {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    fn add_message(&mut self, msg: MessageId) {
        self.messages.push(SequenceEntry::Message(msg));
    }

    fn add_noise(&mut self) {
        if let Some(last) = self.messages.last_mut() {
            if let SequenceEntry::Noise(cnt) = last {
                *cnt += 1;
            } else {
                self.messages.push(SequenceEntry::Noise(1))
            }
        }
    }
}

enum SequenceEntry {
    Message(MessageId),
    Noise(u32), // the number of consecutive noises
}

impl Display for SequenceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SequenceEntry::Message(msg) => msg.to_string(),
            SequenceEntry::Noise(cnt) => format!("-{cnt}"),
        };
        f.write_str(s.as_str())
    }
}
