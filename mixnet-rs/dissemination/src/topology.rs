use std::collections::{HashMap, HashSet};

use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::node::NodeId;

pub fn build_topology(
    num_nodes: u16,
    peering_degree: u16,
    seed: u64,
) -> HashMap<NodeId, HashSet<NodeId>> {
    let mut rng = StdRng::seed_from_u64(seed);

    loop {
        let mut topology: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        for node in 0..num_nodes {
            topology.insert(node, HashSet::new());
        }

        for node in 0..num_nodes {
            let mut others: Vec<NodeId> = Vec::new();
            for other in (0..node).chain(node + 1..num_nodes) {
                // Check if the other node is not already connected to the current node
                // and the other node has not reached the peering degree.
                if !topology.get(&node).unwrap().contains(&other)
                    && topology.get(&other).unwrap().len() < peering_degree as usize
                {
                    others.push(other);
                }
            }

            // How many more connections the current node needs
            let num_needs = peering_degree as usize - topology.get(&node).unwrap().len();
            // Smaple peers as many as possible and connect them to the current node
            let k = std::cmp::min(num_needs, others.len());
            others.as_mut_slice().shuffle(&mut rng);
            others.into_iter().take(k).for_each(|peer| {
                topology.get_mut(&node).unwrap().insert(peer);
                topology.get_mut(&peer).unwrap().insert(node);
            });
        }

        if are_all_nodes_connected(&topology) {
            return topology;
        }
    }
}

fn are_all_nodes_connected(topology: &HashMap<NodeId, HashSet<NodeId>>) -> bool {
    let start_node = topology.keys().next().unwrap();
    let visited = dfs(topology, *start_node);
    visited.len() == topology.len()
}

fn dfs(topology: &HashMap<NodeId, HashSet<NodeId>>, start_node: NodeId) -> HashSet<NodeId> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<NodeId> = Vec::new();
    stack.push(start_node);
    while let Some(node) = stack.pop() {
        visited.insert(node);
        for peer in topology.get(&node).unwrap().iter() {
            if !visited.contains(peer) {
                stack.push(*peer);
            }
        }
    }
    visited
}
