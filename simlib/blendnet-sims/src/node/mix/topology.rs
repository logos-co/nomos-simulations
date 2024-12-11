use std::collections::{HashMap, HashSet};

use netrunner::node::NodeId;
use rand::{seq::SliceRandom, RngCore};

pub type Topology = HashMap<NodeId, HashSet<NodeId>>;

pub fn build_topology<R: RngCore>(nodes: &[NodeId], peering_degree: usize, mut rng: R) -> Topology {
    tracing::info!("Building topology: peering_degree:{}", peering_degree);
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

        let all_connected = check_all_connected(&topology);
        let all_have_peering_degree = check_peering_degree(&topology, peering_degree);
        if all_connected && all_have_peering_degree {
            tracing::info!("Topology built successfully");
            return topology;
        } else {
            tracing::info!(
                "Retrying to build topology: all_connected:{}, all_have_peering_degree:{}",
                all_connected,
                all_have_peering_degree
            );
        }
    }
}

fn check_all_connected(topology: &Topology) -> bool {
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

fn check_peering_degree(topology: &Topology, peering_degree: usize) -> bool {
    topology
        .iter()
        .all(|(_, peers)| peers.len() == peering_degree)
}

#[cfg(test)]
mod tests {
    use netrunner::node::NodeIdExt;

    use super::*;

    #[test]
    fn test_build_topology() {
        tracing_subscriber::fmt::init();

        let nodes = (0..100).map(NodeId::from_index).collect::<Vec<_>>();
        let peering_degree = 3;
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
}
