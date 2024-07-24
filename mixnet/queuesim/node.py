from __future__ import annotations

from typing import Awaitable, Callable

from framework.framework import Framework
from protocol.connection import SimplexConnection
from protocol.node import connect_nodes
from protocol.nomssip import Nomssip, NomssipConfig


class Node:
    def __init__(
        self,
        framework: Framework,
        nomssip_config: NomssipConfig,
        msg_handler: Callable[[bytes], Awaitable[None]],
    ):
        self.nomssip = Nomssip(framework, nomssip_config, msg_handler)

    def connect(
        self,
        peer: Node,
        inbound_conn: SimplexConnection,
        outbound_conn: SimplexConnection,
    ):
        connect_nodes(self.nomssip, peer.nomssip, inbound_conn, outbound_conn)

    async def send_message(self, msg: bytes):
        """
        Send the message via Nomos Gossip to all connected peers.
        """
        await self.nomssip.gossip(msg)
