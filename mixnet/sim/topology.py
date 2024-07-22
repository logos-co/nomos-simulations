import random
from collections import defaultdict

from protocol.node import Node

Topology = dict[int, set[int]]


def build_full_random_topology(
    rng: random.Random, num_nodes: int, peering_degree: int
) -> Topology:
    """
    Generate a random undirected topology until all nodes are connected.
    We don't implement any artificial tool to ensure the connectivity of the topology.
    Instead, we regenerate a topology in a fully randomized way until all nodes are connected.
    """
    while True:
        topology: Topology = defaultdict(set[int])
        nodes = list(range(num_nodes))
        for node in nodes:
            # Filter nodes that can be connected to the current node.
            others = []
            for other in nodes[:node] + nodes[node + 1 :]:
                if len(topology[other]) < peering_degree:
                    others.append(other)
            # How many more connections the current node needs
            n_needs = peering_degree - len(topology[node])
            # Sample peers as many as possible
            peers = rng.sample(others, k=min(n_needs, len(others)))
            # Connect the current node to the peers
            topology[node].update(peers)
            # Connect the peers to the current node, since the topology is undirected
            for peer in peers:
                topology[peer].update([node])

        if are_all_nodes_connected(topology):
            return topology


def are_all_nodes_connected(topology: Topology) -> bool:
    visited = set()

    def dfs(topology: Topology, node: int) -> None:
        if node in visited:
            return
        visited.add(node)
        for peer in topology[node]:
            dfs(topology, peer)

    # Start DFS from the first node
    dfs(topology, next(iter(topology)))

    return len(visited) == len(topology)
