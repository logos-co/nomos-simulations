from __future__ import annotations

from dataclasses import dataclass
from typing import Awaitable, Callable, Generic, Protocol, TypeVar

from framework import Framework
from protocol.connection import (
    DuplexConnection,
    SimplexConnection,
)
from protocol.error import PeeringDegreeReached


@dataclass
class GossipConfig:
    # Expected number of peers each node must connect to if there are enough peers available in the network.
    peering_degree: int


class HasId(Protocol):
    def id(self) -> int: ...


T = TypeVar("T", bound=HasId)


class Gossip(Generic[T]):
    """
    A gossip channel that broadcasts messages to all connected peers.
    Peers are connected via DuplexConnection.
    """

    def __init__(
        self,
        framework: Framework,
        config: GossipConfig,
        handler: Callable[[T], Awaitable[None]],
    ):
        self.framework = framework
        self.config = config
        self.conns: list[DuplexConnection[T]] = []
        # A handler to process inbound messages.
        self.handler = handler
        # msg_id -> received_cnt
        self.packet_cache: dict[int, int] = dict()
        # A set just for gathering a reference of tasks to prevent them from being garbage collected.
        # https://docs.python.org/3/library/asyncio-task.html#asyncio.create_task
        self.tasks: set[Awaitable] = set()

    def can_accept_conn(self) -> bool:
        return len(self.conns) < self.config.peering_degree

    def add_conn(self, inbound: SimplexConnection[T], outbound: SimplexConnection[T]):
        if not self.can_accept_conn():
            # For simplicity of the spec, reject the connection if the peering degree is reached.
            raise PeeringDegreeReached()

        conn = DuplexConnection[T](
            inbound,
            outbound,
        )
        self.conns.append(conn)
        task = self.framework.spawn(self.__process_inbound_conn(conn))
        self.tasks.add(task)

    async def __process_inbound_conn(self, conn: DuplexConnection[T]):
        while True:
            msg = await conn.recv()
            if self._check_update_cache(msg):
                continue
            await self._process_inbound_msg(msg, conn)

    async def _process_inbound_msg(self, msg: T, received_from: DuplexConnection[T]):
        await self._gossip(msg, [received_from])
        await self.handler(msg)

    async def publish(self, msg: T):
        """
        Publish a message to all nodes in the network.
        """
        # Don't publish the same message twice.
        # Touching the cache here is necessary because this method is called by the user,
        # even though we update the cache in the _process_inbound_msg method.
        # It's because we don't want this publisher node to gossip the message again
        # when it first receives the messages from one of its peers later.
        if not self._check_update_cache(msg, publishing=True):
            await self._gossip(msg)
            # With the same reason, call the handler here
            # which means that we consider that this publisher node received the message.
            await self.handler(msg)

    async def _gossip(self, msg: T, excludes: list[DuplexConnection] = []):
        """
        Gossip a message to all peers connected to this node.
        """
        for conn in self.conns:
            if conn not in excludes:
                await conn.send(msg)

    def _check_update_cache(self, msg: T, publishing: bool = False) -> bool:
        """
        Add a message to the cache, and return True if the message was already in the cache.
        """
        id = msg.id()
        seen = id in self.packet_cache

        if publishing:
            if not seen:
                # Put 0 when publishing, so that the publisher node doesn't gossip the message again
                # even when it first receive the message from one of its peers later.
                self.packet_cache[id] = 0
        else:
            if not seen:
                self.packet_cache[id] = 1
            else:
                self.packet_cache[id] += 1
                # Remove the message from the cache if it's received from all adjacent peers in the end
                # to reduce the size of cache.
                if self.packet_cache[id] >= self.config.peering_degree:
                    del self.packet_cache[id]

        return seen
