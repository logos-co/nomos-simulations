from __future__ import annotations

import abc
from typing import Generic, TypeVar

from framework import Framework, Queue
from protocol.temporalmix import TemporalMix, TemporalMixConfig

T = TypeVar("T")


class SimplexConnection(abc.ABC, Generic[T]):
    """
    An abstract class for a simplex connection that can send and receive data in one direction
    """

    @abc.abstractmethod
    async def send(self, data: T) -> None:
        pass

    @abc.abstractmethod
    async def recv(self) -> T:
        pass


class LocalSimplexConnection(SimplexConnection[T]):
    """
    A simplex connection that doesn't have any network latency.
    Data sent through this connection can be immediately received from the other end.
    """

    def __init__(self, framework: Framework):
        self.queue: Queue[T] = framework.queue()

    async def send(self, data: T) -> None:
        await self.queue.put(data)

    async def recv(self) -> T:
        return await self.queue.get()


class DuplexConnection(Generic[T]):
    """
    A duplex connection in which data can be transmitted and received simultaneously in both directions.
    This is to mimic duplex communication in a real network (such as TCP or QUIC).
    """

    def __init__(self, inbound: SimplexConnection[T], outbound: SimplexConnection[T]):
        self.inbound = inbound
        self.outbound = outbound

    async def recv(self) -> T:
        return await self.inbound.recv()

    async def send(self, packet: T):
        await self.outbound.send(packet)


class MixSimplexConnection(SimplexConnection[T]):
    """
    Wraps a SimplexConnection to add a transmission rate and noise to the connection.
    """

    def __init__(
        self,
        framework: Framework,
        conn: SimplexConnection[T],
        transmission_rate_per_sec: int,
        noise_msg: T,
        temporal_mix_config: TemporalMixConfig,
        # OPTIMIZATION ONLY FOR EXPERIMENTS WITHOUT BANDWIDTH MEASUREMENT
        # If True, skip sending a noise even if it's time to send one.
        skip_sending_noise: bool,
    ):
        self.framework = framework
        self.queue: Queue[T] = TemporalMix.queue(
            temporal_mix_config, framework, noise_msg
        )
        self.conn = conn
        self.transmission_rate_per_sec = transmission_rate_per_sec
        self.noise_msg = noise_msg
        self.skip_sending_noise = skip_sending_noise
        self.task = framework.spawn(self.__run())

    async def __run(self):
        while True:
            await self.framework.sleep(1 / self.transmission_rate_per_sec)
            msg = await self.queue.get()
            if self.skip_sending_noise and msg == self.noise_msg:
                continue
            await self.conn.send(msg)

    async def send(self, data: T) -> None:
        await self.queue.put(data)

    async def recv(self) -> T:
        return await self.conn.recv()
