import math
import random
from unittest import IsolatedAsyncioTestCase

import usim

import framework.usim as usimfw
from protocol.connection import LocalSimplexConnection
from protocol.node import Node
from protocol.test_utils import (
    init_mixnet_config,
)
from sim.config import LatencyConfig, NetworkConfig
from sim.connection import (
    MeteredRemoteSimplexConnection,
    ObservedMeteredRemoteSimplexConnection,
)
from sim.state import NodeState, NodeStateTable


class TestMeteredRemoteSimplexConnection(IsolatedAsyncioTestCase):
    async def test_latency(self):
        usim.run(self.__test_latency())

    async def __test_latency(self):
        async with usim.Scope() as scope:
            framework = usimfw.Framework(scope)
            node_state_table = NodeStateTable(num_nodes=2, duration_sec=3)
            conn = MeteredRemoteSimplexConnection(
                LatencyConfig(
                    min_latency_sec=0,
                    max_latency_sec=1,
                    seed=random.Random(),
                ),
                framework,
                framework.now(),
            )

            # Send two messages without delay
            sent_time = framework.now()
            await conn.send(b"hello")
            await conn.send(b"world")

            # Receive two messages and check if the network latency was simulated well.
            # There should be no delay between the two messages because they were sent without delay.
            self.assertEqual(b"hello", await conn.recv())
            self.assertEqual(conn.latency, framework.now() - sent_time)
            self.assertEqual(b"world", await conn.recv())
            self.assertEqual(conn.latency, framework.now() - sent_time)


class TestObservedMeteredRemoteSimplexConnection(IsolatedAsyncioTestCase):
    async def test_node_state(self):
        usim.run(self.__test_node_state())

    async def __test_node_state(self):
        async with usim.Scope() as scope:
            framework = usimfw.Framework(scope)
            node_state_table = NodeStateTable(num_nodes=2, duration_sec=3)
            meter_start_time = framework.now()
            conn = ObservedMeteredRemoteSimplexConnection(
                LatencyConfig(
                    min_latency_sec=0,
                    max_latency_sec=1,
                    seed=random.Random(),
                ),
                framework,
                meter_start_time,
                node_state_table[0],
                node_state_table[1],
            )

            # Sleep and send a message
            await framework.sleep(1)
            sent_time = framework.now()
            await conn.send(b"hello")

            # Receive the message. It should be received after the latency.
            self.assertEqual(b"hello", await conn.recv())
            recv_time = framework.now()

            # Check if the sender node state is SENDING at the sent time
            timeslot = math.floor((sent_time - meter_start_time) * 1000)
            self.assertEqual(
                NodeState.SENDING,
                node_state_table[0][timeslot],
            )
            # Ensure that the sender node states in other time slots are IDLE
            states = set()
            states.update(node_state_table[0][:timeslot])
            states.update(node_state_table[0][timeslot + 1 :])
            self.assertEqual(set([NodeState.IDLE]), states)

            # Check if the receiver node state is RECEIVING at the received time
            timeslot = math.floor((recv_time - meter_start_time) * 1000)
            self.assertEqual(
                NodeState.RECEIVING,
                node_state_table[1][timeslot],
            )
            # Ensure that the receiver node states in other time slots are IDLE
            states = set()
            states.update(node_state_table[1][:timeslot])
            states.update(node_state_table[1][timeslot + 1 :])
            self.assertEqual(set([NodeState.IDLE]), states)
