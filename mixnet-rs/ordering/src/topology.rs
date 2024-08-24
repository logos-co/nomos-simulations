use std::{fmt::Debug, hash::Hash};

use protocol::{
    node::{Node, NodeId},
    queue::QueueConfig,
    topology::build_topology,
};
use rand::{rngs::StdRng, seq::SliceRandom, RngCore, SeedableRng};
use rustc_hash::FxHashMap;

use crate::{outputs::Outputs, paramset::ParamSet};

pub const RECEIVER_NODE_ID: NodeId = NodeId::MAX;

pub fn build_striped_network<M: 'static + Debug + Copy + Clone + PartialEq + Eq + Hash>(
    paramset: &ParamSet,
    seed: u64,
) -> (Vec<Node<M>>, Vec<Vec<NodeId>>, FxHashMap<NodeId, u16>) {
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
    let mut receiver_peer_conn_idx: FxHashMap<NodeId, u16> = FxHashMap::default();
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

                receiver_peer_conn_idx
                    .insert(*id, receiver_peer_conn_idx.len().try_into().unwrap());
            }
        }
    }
    let sender_peers_list: Vec<Vec<NodeId>> =
        vec![
            paths.iter().map(|path| *path.first().unwrap()).collect();
            paramset.num_senders as usize
        ];
    (mixnodes, sender_peers_list, receiver_peer_conn_idx)
}

pub fn build_random_network<M: 'static + Debug + Copy + Clone + PartialEq + Eq + Hash>(
    paramset: &ParamSet,
    seed: u64,
    outputs: &mut Outputs,
) -> (Vec<Node<M>>, Vec<Vec<NodeId>>, FxHashMap<NodeId, u16>) {
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
    let mut sender_peers_list: Vec<Vec<NodeId>> = Vec::with_capacity(paramset.num_senders as usize);
    for _ in 0..paramset.num_senders {
        candidates.as_mut_slice().shuffle(&mut peers_rng);
        let mut peers: Vec<NodeId> = candidates
            .iter()
            .cloned()
            .take(paramset.peering_degree as usize)
            .collect();
        peers.sort();
        sender_peers_list.push(peers);
    }
    candidates.as_mut_slice().shuffle(&mut peers_rng);
    let mut receiver_peers: Vec<NodeId> = candidates
        .iter()
        .cloned()
        .take(paramset.peering_degree as usize)
        .collect();
    receiver_peers.sort();

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
    //
    // peer_id -> conn_idx
    let mut receiver_peer_conn_idx: FxHashMap<NodeId, u16> = FxHashMap::default();
    for (conn_idx, mixnode_id) in receiver_peers.iter().enumerate() {
        let mixnode = mixnodes.get_mut(*mixnode_id as usize).unwrap();
        assert_eq!(mixnode.id, *mixnode_id);
        mixnode.connect(RECEIVER_NODE_ID);

        receiver_peer_conn_idx.insert(*mixnode_id, conn_idx.try_into().unwrap());
    }

    outputs.write_topology(&topology, &sender_peers_list, &receiver_peers);

    (mixnodes, sender_peers_list, receiver_peer_conn_idx)
}
