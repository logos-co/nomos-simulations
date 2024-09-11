from __future__ import annotations

import hashlib
from dataclasses import dataclass
from enum import Enum
from typing import Awaitable, Callable, Self

from framework import Framework
from protocol.connection import (
    DuplexConnection,
    MixSimplexConnection,
    SimplexConnection,
)
from protocol.error import PeeringDegreeReached


@dataclass
class GossipConfig:
    # Expected number of peers each node must connect to if there are enough peers available in the network.
    peering_degree: int


class Gossip:
    """
    A gossip channel that broadcasts messages to all connected peers.
    Peers are connected via DuplexConnection.
    """

    def __init__(
        self,
        framework: Framework,
        config: GossipConfig,
        handler: Callable[[bytes], Awaitable[None]],
    ):
        self.framework = framework
        self.config = config
        self.conns: list[DuplexConnection] = []
        # A handler to process inbound messages.
        self.handler = handler
        self.packet_cache: set[bytes] = set()
        # A set just for gathering a reference of tasks to prevent them from being garbage collected.
        # https://docs.python.org/3/library/asyncio-task.html#asyncio.create_task
        self.tasks: set[Awaitable] = set()

    def can_accept_conn(self) -> bool:
        return len(self.conns) < self.config.peering_degree

    def add_conn(self, inbound: SimplexConnection, outbound: SimplexConnection):
        if not self.can_accept_conn():
            # For simplicity of the spec, reject the connection if the peering degree is reached.
            raise PeeringDegreeReached()

        conn = DuplexConnection(
            inbound,
            outbound,
        )
        self.conns.append(conn)
        task = self.framework.spawn(self.__process_inbound_conn(conn))
        self.tasks.add(task)

    async def __process_inbound_conn(self, conn: DuplexConnection):
        while True:
            msg = await conn.recv()
            if self.__check_update_cache(msg):
                continue
            await self.process_inbound_msg(msg)

    async def process_inbound_msg(self, msg: bytes):
        await self.gossip(msg)
        await self.handler(msg)

    async def gossip(self, msg: bytes):
        """
        Gossip a message to all connected peers.
        """
        for conn in self.conns:
            await conn.send(msg)

    def __check_update_cache(self, packet: bytes) -> bool:
        """
        Add a message to the cache, and return True if the message was already in the cache.
        """
        hash = hashlib.sha256(packet).digest()
        if hash in self.packet_cache:
            return True
        self.packet_cache.add(hash)
        return False
