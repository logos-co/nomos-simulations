use std::{fmt::Debug, hash::Hash};

use protocol::{
    node::{Node, NodeId},
    queue::QueueConfig,
    topology::build_topology,
};
use rand::{rngs::StdRng, seq::SliceRandom, RngCore, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{message::SenderIdx, outputs::Outputs, paramset::ParamSet};

pub const RECEIVER_NODE_ID: NodeId = NodeId::MAX;

pub fn build_striped_network<M: 'static + Debug + Copy + Clone + PartialEq + Eq + Hash>(
    paramset: &ParamSet,
    seed: u64,
) -> (Vec<Node<M>>, AllSenderPeers, ReceiverPeers) {
    assert!(!paramset.random_topology);
    let mut next_node_id: NodeId = 0;
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    let mut mixnodes: Vec<Node<M>> =
        Vec::with_capacity(paramset.num_paths as usize * paramset.num_mixes as usize);
    let mut paths: Vec<Vec<NodeId>> = Vec::with_capacity(paramset.num_paths as usize);
    for _ in 0..paramset.num_paths {
        let mut ids = Vec::with_capacity(paramset.num_mixes as usize);
        for _ in 0..paramset.num_mixes {
            let id = next_node_id;
            next_node_id += 1;
            mixnodes.push(Node::new(
                id,
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
    let mut receiver_peers = ReceiverPeers::new();
    for path in paths.iter() {
        for (i, id) in path.iter().enumerate() {
            if i != path.len() - 1 {
                let peer_id = path[i + 1];
                let mixnode = mixnodes.get_mut(*id as usize).unwrap();
                assert_eq!(mixnode.id, *id);
                mixnode.connect(peer_id);
            } else {
                let mixnode = mixnodes.get_mut(*id as usize).unwrap();
                assert_eq!(mixnode.id, *id);
                mixnode.connect(RECEIVER_NODE_ID);

                receiver_peers.add(*id, receiver_peers.len());
            }
        }
    }

    let mut all_sender_peers = AllSenderPeers::new(paramset.num_senders);
    let sender_peers = paths
        .iter()
        .map(|path| *path.first().unwrap())
        .collect::<Vec<_>>();
    (0..paramset.num_senders).for_each(|_| {
        all_sender_peers.add(sender_peers.clone());
    });

    (mixnodes, all_sender_peers, receiver_peers)
}

pub fn build_random_network<M: 'static + Debug + Copy + Clone + PartialEq + Eq + Hash>(
    paramset: &ParamSet,
    seed: u64,
    outputs: &mut Outputs,
) -> (Vec<Node<M>>, AllSenderPeers, ReceiverPeers) {
    assert!(paramset.random_topology);
    // Init mix nodes
    let mut queue_seed_rng = StdRng::seed_from_u64(seed);
    let mut mixnodes: Vec<Node<M>> = Vec::with_capacity(paramset.num_mixes as usize);
    for id in 0..paramset.num_mixes {
        mixnodes.push(Node::new(
            id,
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
    let mut candidates: Vec<NodeId> = mixnodes.iter().map(|mixnode| mixnode.id).collect();
    assert!(candidates.len() >= paramset.peering_degree as usize);
    let mut all_sender_peers = AllSenderPeers::new(paramset.num_senders);
    for _ in 0..paramset.num_senders {
        candidates.as_mut_slice().shuffle(&mut peers_rng);
        let mut peers: Vec<NodeId> = candidates
            .iter()
            .cloned()
            .take(paramset.peering_degree as usize)
            .collect();
        peers.sort();
        all_sender_peers.add(peers);
    }
    candidates.as_mut_slice().shuffle(&mut peers_rng);
    let mut receiver_peer_ids: Vec<NodeId> = candidates
        .iter()
        .cloned()
        .take(paramset.peering_degree as usize)
        .collect();
    receiver_peer_ids.sort();

    // Connect mix nodes
    let topology = build_topology(
        paramset.num_mixes,
        &vec![paramset.peering_degree; paramset.num_mixes as usize],
        seed,
    );
    for (node_id, peers) in topology.iter().enumerate() {
        peers.iter().for_each(|peer_id| {
            let mixnode = mixnodes.get_mut(node_id).unwrap();
            assert_eq!(mixnode.id as usize, node_id);
            mixnode.connect(*peer_id);
        });
    }

    // Connect the selected mix nodes with the receiver
    let mut receiver_peers = ReceiverPeers::new();
    for (conn_idx, mixnode_id) in receiver_peer_ids.iter().enumerate() {
        let mixnode = mixnodes.get_mut(*mixnode_id as usize).unwrap();
        assert_eq!(mixnode.id, *mixnode_id);
        mixnode.connect(RECEIVER_NODE_ID);

        receiver_peers.add(*mixnode_id, conn_idx);
    }

    outputs.write_topology(&topology, &all_sender_peers, &receiver_peer_ids);

    (mixnodes, all_sender_peers, receiver_peers)
}

pub struct AllSenderPeers(Vec<Vec<NodeId>>);

impl AllSenderPeers {
    fn new(num_senders: u8) -> Self {
        Self(Vec::with_capacity(num_senders as usize))
    }

    fn add(&mut self, peers: Vec<NodeId>) {
        self.0.push(peers)
    }

    pub fn iter(&self) -> impl Iterator<Item = (SenderIdx, &Vec<NodeId>)> {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, v)| (idx.try_into().unwrap(), v))
    }
}

pub struct ReceiverPeers(FxHashMap<NodeId, usize>);

impl ReceiverPeers {
    fn new() -> Self {
        ReceiverPeers(FxHashMap::default())
    }

    fn add(&mut self, peer_id: NodeId, conn_idx: usize) {
        self.0.insert(peer_id, conn_idx);
    }

    pub fn conn_idx(&self, node_id: &NodeId) -> Option<usize> {
        self.0.get(node_id).cloned()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}
