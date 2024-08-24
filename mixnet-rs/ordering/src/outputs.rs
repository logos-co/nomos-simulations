use std::{fs::File, path::Path};

use protocol::{
    node::{Node, NodeId},
    topology::Topology,
};

use crate::{message::DataMessage, ordercoeff::SequenceWriter};

pub struct Outputs {
    closed: bool,
    // gradual writing
    latency_path: String,
    latency_writer: csv::Writer<File>,
    sent_sequence_paths: Vec<String>,
    sent_sequence_writers: Vec<SequenceWriter>,
    recv_sequence_paths: Vec<String>,
    recv_sequence_writers: Vec<SequenceWriter>,
    queue_data_msg_counts_path: String,
    queue_data_msg_counts_writer: csv::Writer<File>,
    // bulk writing
    pub topology_path: String,
}

impl Outputs {
    pub fn new(
        latency_path: String,
        sent_sequence_paths: Vec<String>,
        recv_sequence_paths: Vec<String>,
        queue_data_msg_counts_path: String,
        topology_path: String,
    ) -> Self {
        // Ensure that all output files do not exist
        for path in [
            latency_path.clone(),
            queue_data_msg_counts_path.clone(),
            topology_path.clone(),
        ]
        .iter()
        .chain(sent_sequence_paths.iter())
        .chain(recv_sequence_paths.iter())
        {
            assert!(!Path::new(path).exists(), "File already exists: {path}");
        }

        // Prepare writers and headers
        let mut latency_writer = csv::Writer::from_path(&latency_path).unwrap();
        latency_writer
            .write_record(["msg", "latency", "sent_time", "recv_time"])
            .unwrap();
        latency_writer.flush().unwrap();
        let sent_sequence_writers = sent_sequence_paths
            .iter()
            .map(|path| SequenceWriter::new(path))
            .collect::<Vec<_>>();
        let recv_sequence_writers = recv_sequence_paths
            .iter()
            .map(|path| SequenceWriter::new(path))
            .collect::<Vec<_>>();
        let queue_data_msg_counts_writer =
            csv::Writer::from_path(&queue_data_msg_counts_path).unwrap();

        Self {
            closed: false,
            latency_path,
            latency_writer,
            sent_sequence_paths,
            sent_sequence_writers,
            recv_sequence_paths,
            recv_sequence_writers,
            queue_data_msg_counts_path,
            queue_data_msg_counts_writer,
            topology_path,
        }
    }

    pub fn close(&mut self) {
        self.latency_writer.flush().unwrap();
        for seq in &mut self.sent_sequence_writers {
            seq.flush();
        }
        for seq in &mut self.recv_sequence_writers {
            seq.flush();
        }
        self.queue_data_msg_counts_writer.flush().unwrap();

        self.closed = true;
    }

    pub fn add_latency(&mut self, msg: &DataMessage, sent_time: f32, recv_time: f32) {
        self.latency_writer
            .write_record(&[
                msg.to_string(),
                (recv_time - sent_time).to_string(),
                sent_time.to_string(),
                recv_time.to_string(),
            ])
            .unwrap();
    }

    pub fn add_sent_msg(&mut self, msg: &DataMessage) {
        let writer = &mut self.sent_sequence_writers[msg.sender as usize];
        writer.add_message(msg);
    }

    pub fn add_sent_noise(&mut self, sender_idx: usize) {
        let writer = &mut self.sent_sequence_writers[sender_idx];
        writer.add_noise();
    }

    pub fn add_recv_msg(&mut self, msg: &DataMessage, conn_idx: usize) {
        let writer = &mut self.recv_sequence_writers[conn_idx];
        writer.add_message(msg);
    }

    pub fn add_recv_noise(&mut self, conn_idx: usize) {
        let writer = &mut self.recv_sequence_writers[conn_idx];
        writer.add_noise();
    }

    pub fn write_header_queue_data_msg_counts(&mut self, mixnodes: &[Node<DataMessage>]) {
        let writer = &mut self.queue_data_msg_counts_writer;
        let mut header = vec!["vtime".to_string()];
        mixnodes
            .iter()
            .map(|node| (node.id, node.queue_data_msg_counts()))
            .for_each(|(node_id, counts)| {
                let num_queues = counts.len();
                (0..num_queues).for_each(|q_idx| {
                    header.push(format!("node{}_q{}", node_id, q_idx));
                });
            });
        writer.write_record(header).unwrap();
        writer.flush().unwrap();
    }

    pub fn add_queue_data_msg_counts(&mut self, vtime: f32, mixnodes: &[Node<DataMessage>]) {
        let writer = &mut self.queue_data_msg_counts_writer;
        let mut record = vec![vtime.to_string()];
        mixnodes
            .iter()
            .map(|node| node.queue_data_msg_counts())
            .for_each(|counts| {
                counts.iter().for_each(|count| {
                    record.push(count.to_string());
                });
            });
        writer.write_record(record).unwrap();
    }

    pub fn write_topology(
        &self,
        topology: &Topology,
        sender_peers_list: &[Vec<NodeId>],
        receiver_peers: &[NodeId],
    ) {
        let mut writer = csv::Writer::from_path(&self.topology_path).unwrap();
        writer.write_record(["node", "num_peers", "peers"]).unwrap();

        // Write peers of mix nodes
        for (node_id, peers) in topology.iter().enumerate() {
            writer
                .write_record(&[
                    node_id.to_string(),
                    peers.len().to_string(),
                    format!(
                        "[{}]",
                        peers
                            .iter()
                            .map(|peer_id| peer_id.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                ])
                .unwrap();
        }

        // Write peers of senders
        for (sender_idx, peers) in sender_peers_list.iter().enumerate() {
            writer
                .write_record(&[
                    format!("sender-{}", sender_idx),
                    peers.len().to_string(),
                    format!(
                        "[{}]",
                        peers
                            .iter()
                            .map(|peer_id| peer_id.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                ])
                .unwrap();
        }

        // Write peers of the receiver
        writer
            .write_record(&[
                "receiver".to_string(),
                receiver_peers.len().to_string(),
                format!(
                    "[{}]",
                    receiver_peers
                        .iter()
                        .map(|peer_id| peer_id.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ])
            .unwrap();

        writer.flush().unwrap();
    }

    pub fn rename_paths(&self, from: &str, to: &str) {
        assert!(self.closed);

        for path in [
            &self.latency_path.clone(),
            &self.queue_data_msg_counts_path.clone(),
        ]
        .into_iter()
        .chain(self.sent_sequence_paths.iter())
        .chain(self.recv_sequence_paths.iter())
        {
            let new_path = path.replace(from, to);
            std::fs::rename(path, new_path).unwrap();
        }
    }
}
