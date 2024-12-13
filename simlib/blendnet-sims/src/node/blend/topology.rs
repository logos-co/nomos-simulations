use std::collections::{HashMap, HashSet};

use netrunner::node::NodeId;
use rand::{seq::SliceRandom, RngCore};

pub type Topology = HashMap<NodeId, HashSet<NodeId>>;

/// Builds a topology with the given nodes and peering degree
/// by ensuring that all nodes are connected (no partition)
/// and all nodes have the same number of connections (only if possible).
pub fn build_topology<R: RngCore>(nodes: &[NodeId], peering_degree: usize, mut rng: R) -> Topology {
    tracing::info!("Building topology: peering_degree:{}", peering_degree);
    loop {
        let mut topology = nodes
            .iter()
            .map(|&node| (node, HashSet::new()))
            .collect::<HashMap<_, _>>();

        for node in nodes.iter() {
            // Collect peer candidates
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

        // Check constraints:
        // - All nodes are connected (no partition)
        // - All nodes have the same number of connections (if possible)
        let can_have_equal_conns = (nodes.len() * peering_degree) % 2 == 0;
        if check_all_connected(&topology)
            && (!can_have_equal_conns || check_equal_conns(&topology, peering_degree))
        {
            return topology;
        }
        tracing::info!("Topology doesn't meet constraints. Retrying...");
    }
}

/// Checks if all nodes are connected (no partition) in the topology.
fn check_all_connected(topology: &Topology) -> bool {
    let visited = dfs(topology, *topology.keys().next().unwrap());
    visited.len() == topology.len()
}

/// Depth-first search to visit nodes in the topology.
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

/// Checks if all nodes have the same number of connections.
fn check_equal_conns(topology: &Topology, peering_degree: usize) -> bool {
    topology
        .iter()
        .all(|(_, peers)| peers.len() == peering_degree)
}

#[cfg(test)]
mod tests {
    use netrunner::node::NodeIdExt;

    use super::*;

    #[test]
    fn test_build_topology_with_equal_conns() {
        // If num_nodes * peering_degree is even,
        // it is possible that all nodes can have the same number of connections
        let nodes = (0..7).map(NodeId::from_index).collect::<Vec<_>>();
        let peering_degree = 4;

        let mut rng = rand::rngs::OsRng;
        let topology = build_topology(&nodes, peering_degree, &mut rng);
        assert_eq!(topology.len(), nodes.len());
        for (node, peers) in topology.iter() {
            assert!(peers.len() == peering_degree);
            for peer in peers.iter() {
                assert!(topology.get(peer).unwrap().contains(node));
            }
        }
    }

    #[test]
    fn test_build_topology_with_inequal_conns() {
        // If num_nodes * peering_degree is odd,
        // it is impossible that all nodes can have the same number of connections
        let nodes = (0..7).map(NodeId::from_index).collect::<Vec<_>>();
        let peering_degree = 3;

        let mut rng = rand::rngs::OsRng;
        let topology = build_topology(&nodes, peering_degree, &mut rng);
        assert_eq!(topology.len(), nodes.len());
        for (node, peers) in topology.iter() {
            assert!(peers.len() <= peering_degree);
            for peer in peers.iter() {
                assert!(topology.get(peer).unwrap().contains(node));
            }
        }
    }
}
