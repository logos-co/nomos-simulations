import math
from collections import Counter
from typing import Awaitable

import pandas

from framework import Framework, Queue
from protocol.connection import SimplexConnection
from sim.config import LatencyConfig, NetworkConfig
from sim.state import NodeState


class MeteredRemoteSimplexConnection(SimplexConnection):
    """
    A simplex connection implementation that simulates network latency and measures bandwidth usages.
    """

    def __init__(
        self,
        config: LatencyConfig,
        framework: Framework,
        meter_start_time: float,
        send_node_states: list[NodeState],
        recv_node_states: list[NodeState],
    ):
        self.framework = framework
        # A connection has a random constant latency
        self.latency = config.random_latency()
        # A queue of tuple(timestamp, msg) where a sender puts messages to be sent
        self.send_queue: Queue[tuple[float, bytes]] = framework.queue()
        # A task that reads messages from send_queue, and puts them to recv_queue.
        # Before putting messages to recv_queue, the task simulates network latency according to the timestamp of each message.
        self.relayer = framework.spawn(self.__run_relayer())
        # A queue where a receiver gets messages
        self.recv_queue: Queue[bytes] = framework.queue()
        # To measure bandwidth usages
        self.meter_start_time = meter_start_time
        self.send_meters: list[int] = []
        self.recv_meters: list[int] = []
        # To measure node states over time
        self.send_node_states = send_node_states
        self.recv_node_states = recv_node_states
        # To measure the size of messages sent via this connection
        self.msg_sizes: Counter[int] = Counter()

    async def send(self, data: bytes) -> None:
        await self.send_queue.put((self.framework.now(), data))
        self.__update_meter(self.send_meters, len(data))
        self.__update_node_state(self.send_node_states, NodeState.SENDING)
        self.msg_sizes.update([len(data)])

    async def recv(self) -> bytes:
        return await self.recv_queue.get()

    async def __run_relayer(self):
        """
        A task that reads messages from send_queue, and puts them to recv_queue.
        Before putting messages to recv_queue, the task simulates network latency according to the timestamp of each message.
        """
        while True:
            sent_time, data = await self.send_queue.get()
            # Simulate network latency
            delay = self.latency - (self.framework.now() - sent_time)
            if delay > 0:
                await self.framework.sleep(delay)

            # Relay msg to the recv_queue.
            # Update meter & node_state before msg is read from recv_queue by the receiver
            # because the time at which enters the node is important when viewed from the outside.
            self.__update_meter(self.recv_meters, len(data))
            self.__update_node_state(self.recv_node_states, NodeState.RECEIVING)
            await self.recv_queue.put(data)

    def __update_meter(self, meters: list[int], size: int):
        """
        Accumulates the bandwidth usage in the current time slot (seconds).
        """
        slot = math.floor(self.framework.now() - self.meter_start_time)
        assert slot >= len(meters) - 1
        # Fill zeros for the empty time slots
        meters.extend([0] * (slot - len(meters) + 1))
        meters[-1] += size

    def __update_node_state(self, node_states: list[NodeState], state: NodeState):
        # The time unit of node states is milliseconds
        ms = math.floor(self.framework.now() * 1000)
        node_states[ms] = state

    def sending_bandwidths(self) -> pandas.Series:
        return self.__bandwidths(self.send_meters)

    def receiving_bandwidths(self) -> pandas.Series:
        return self.__bandwidths(self.recv_meters)

    def __bandwidths(self, meters: list[int]) -> pandas.Series:
        return pandas.Series(meters, name="bandwidth")
