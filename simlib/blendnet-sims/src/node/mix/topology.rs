use std::collections::{HashMap, HashSet};

use netrunner::node::NodeId;
use rand::{seq::SliceRandom, RngCore};

pub type Topology = HashMap<NodeId, HashSet<NodeId>>;

pub fn build_topology<R: RngCore>(nodes: &[NodeId], peering_degree: usize, mut rng: R) -> Topology {
    loop {
        let mut topology = nodes
            .iter()
            .map(|&node| (node, HashSet::new()))
            .collect::<HashMap<_, _>>();

        for node in nodes.iter() {
            let mut others = nodes
                .iter()
                .filter(|&other| {
                    // Check if the other node is not already connected to the current node
                    // and the other node has not reached the peering degree.
                    other != node
                        && !topology.get(node).unwrap().contains(other)
                        && topology.get(other).unwrap().len() < peering_degree
                })
                .copied()
                .collect::<Vec<_>>();

            // How many more connections the current node needs
            let num_needs = peering_degree - topology.get(node).unwrap().len();
            // Sample peers as many as possible and connect them to the current node
            let k = std::cmp::min(num_needs, others.len());
            others.as_mut_slice().shuffle(&mut rng);
            others.into_iter().take(k).for_each(|peer| {
                topology.get_mut(node).unwrap().insert(peer);
                topology.get_mut(&peer).unwrap().insert(*node);
            });
        }

        if are_all_nodes_connected(&topology) {
            return topology;
        }
    }
}

fn are_all_nodes_connected(topology: &Topology) -> bool {
    let visited = dfs(topology, *topology.keys().next().unwrap());
    visited.len() == topology.len()
}

fn dfs(topology: &Topology, start_node: NodeId) -> HashSet<NodeId> {
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
