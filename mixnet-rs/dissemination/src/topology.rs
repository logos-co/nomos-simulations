use std::collections::HashSet;

use protocol::node::NodeId;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

pub type Topology = Vec<Vec<NodeId>>;

pub fn build_topology(num_nodes: u32, peering_degrees: &[u32], seed: u64) -> Topology {
    assert_eq!(num_nodes as usize, peering_degrees.len());
    let mut rng = StdRng::seed_from_u64(seed);

    loop {
        let mut topology: Vec<HashSet<NodeId>> = Vec::new();
        for _ in 0..num_nodes {
            topology.push(HashSet::new());
        }

        for node in 0..num_nodes {
            let mut others: Vec<NodeId> = Vec::new();
            for other in (0..node).chain(node + 1..num_nodes) {
                // Check if the other node is not already connected to the current node
                // and the other node has not reached the peering degree.
                if !topology[node as usize].contains(&other)
                    && topology[other as usize].len() < peering_degrees[other as usize] as usize
                {
                    others.push(other);
                }
            }

            // How many more connections the current node needs
            let num_needs = peering_degrees[node as usize] as usize - topology[node as usize].len();
            // Smaple peers as many as possible and connect them to the current node
            let k = std::cmp::min(num_needs, others.len());
            others.as_mut_slice().shuffle(&mut rng);
            others.into_iter().take(k).for_each(|peer| {
                topology[node as usize].insert(peer);
                topology[peer as usize].insert(node);
            });
        }

        if are_all_nodes_connected(&topology) {
            let mut sorted_topology: Vec<Vec<NodeId>> = Vec::new();
            for peers in topology.iter() {
                let mut sorted_peers: Vec<NodeId> = peers.iter().copied().collect();
                sorted_peers.sort();
                sorted_topology.push(sorted_peers);
            }
            return sorted_topology;
        }
    }
}

fn are_all_nodes_connected(topology: &[HashSet<NodeId>]) -> bool {
    let visited = dfs(topology, 0);
    visited.len() == topology.len()
}

fn dfs(topology: &[HashSet<NodeId>], start_node: NodeId) -> HashSet<NodeId> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<NodeId> = Vec::new();
    stack.push(start_node);
    while let Some(node) = stack.pop() {
        visited.insert(node);
        for peer in topology[node as usize].iter() {
            if !visited.contains(peer) {
                stack.push(*peer);
            }
        }
    }
    visited
}
