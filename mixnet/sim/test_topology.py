import random
from unittest import TestCase

from sim.topology import are_all_nodes_connected, build_full_random_topology


class TestTopology(TestCase):
    def test_full_random(self):
        num_nodes = 10000
        peering_degree = 16
        topology = build_full_random_topology(
            random.Random(0), num_nodes, peering_degree
        )
        self.assertEqual(num_nodes, len(topology))
        self.assertTrue(are_all_nodes_connected(topology))
        for node, peers in topology.items():
            self.assertTrue(0 < len(peers) <= peering_degree)
            # Check if nodes are interconnected
            for peer in peers:
                self.assertIn(node, topology[peer])
