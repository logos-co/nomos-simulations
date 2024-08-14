import math
from collections import Counter
from typing import Protocol, TypeVar

import pandas
from typing_extensions import override

from framework import Framework, Queue
from protocol.connection import SimplexConnection
from sim.config import LatencyConfig
from sim.state import NodeState

T = TypeVar("T")


class RemoteSimplexConnection(SimplexConnection[T]):
    """
    A simplex connection implementation that simulates network latency.
    """

    def __init__(self, config: LatencyConfig, framework: Framework):
        self.framework = framework
        # A connection has a random constant latency
        self.latency = config.random_latency()
        # A queue of tuple(timestamp, msg) where a sender puts messages to be sent
        self.send_queue: Queue[tuple[float, T]] = framework.queue()
        # A task that reads messages from send_queue, and puts them to recv_queue.
        # Before putting messages to recv_queue, the task simulates network latency according to the timestamp of each message.
        self.relayer = framework.spawn(self.__run_relayer())
        # A queue where a receiver gets messages
        self.recv_queue: Queue[T] = framework.queue()

    async def send(self, data: T) -> None:
        await self.send_queue.put((self.framework.now(), data))
        self.on_sending(data)

    async def recv(self) -> T:
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
            # Call on_receiving (e.g. for updating stats) before msg is read from recv_queue by the receiver
            # because the time at which enters the node is important when viewed from the outside.
            self.on_receiving(data)
            await self.recv_queue.put(data)

    def on_sending(self, data: T) -> None:
        # Should be overridden by subclass
        pass

    def on_receiving(self, data: T) -> None:
        # Should be overridden by subclass
        pass


class HasLen(Protocol):
    def __len__(self) -> int: ...


TL = TypeVar("TL", bound=HasLen)


class MeteredRemoteSimplexConnection(RemoteSimplexConnection[TL]):
    """
    An extension of RemoteSimplexConnection that measures bandwidth usages.
    """

    def __init__(
        self,
        config: LatencyConfig,
        framework: Framework,
        meter_start_time: float,
    ):
        super().__init__(config, framework)
        # To measure bandwidth usages
        self.meter_start_time = meter_start_time
        self.send_meters: list[int] = []
        self.recv_meters: list[int] = []

    @override
    def on_sending(self, data: TL) -> None:
        """
        Update statistics when sending a message
        """
        self.__update_meter(self.send_meters, len(data))

    @override
    def on_receiving(self, data: TL) -> None:
        """
        Update statistics when receiving a message
        """
        self.__update_meter(self.recv_meters, len(data))

    def __update_meter(self, meters: list[int], size: int):
        """
        Accumulates the bandwidth usage in the current time slot (seconds).
        """
        slot = math.floor(self.framework.now() - self.meter_start_time)
        assert slot >= len(meters) - 1
        # Fill zeros for the empty time slots
        meters.extend([0] * (slot - len(meters) + 1))
        meters[-1] += size

    def sending_bandwidths(self) -> pandas.Series:
        """
        Returns the accumulated sending bandwidth usage over time
        """
        return self.__bandwidths(self.send_meters)

    def receiving_bandwidths(self) -> pandas.Series:
        """
        Returns the accumulated receiving bandwidth usage over time
        """
        return self.__bandwidths(self.recv_meters)

    def __bandwidths(self, meters: list[int]) -> pandas.Series:
        return pandas.Series(meters, name="bandwidth")


class ObservedMeteredRemoteSimplexConnection(MeteredRemoteSimplexConnection[TL]):
    """
    An extension of MeteredRemoteSimplexConnection that is observed by passive observer.
    The observer monitors the node states of the sender and receiver and message sizes.
    """

    def __init__(
        self,
        config: LatencyConfig,
        framework: Framework,
        meter_start_time: float,
        send_node_states: list[NodeState],
        recv_node_states: list[NodeState],
    ):
        super().__init__(config, framework, meter_start_time)

        # To measure node states over time
        self.send_node_states = send_node_states
        self.recv_node_states = recv_node_states
        # To measure the size of messages sent via this connection
        self.msg_sizes: Counter[int] = Counter()

    @override
    def on_sending(self, data: TL) -> None:
        super().on_sending(data)
        self.__update_node_state(self.send_node_states, NodeState.SENDING)
        self.msg_sizes.update([len(data)])

    @override
    def on_receiving(self, data: TL) -> None:
        super().on_receiving(data)
        self.__update_node_state(self.recv_node_states, NodeState.RECEIVING)

    def __update_node_state(self, node_states: list[NodeState], state: NodeState):
        # The time unit of node states is milliseconds
        ms = math.floor(self.framework.now() * 1000)
        node_states[ms] = state
