from __future__ import annotations

from typing import Awaitable, Callable

from framework.framework import Framework
from protocol.connection import SimplexConnection
from protocol.node import connect_nodes
from protocol.nomssip import Nomssip, NomssipConfig, NomssipMessage
from queuesim.message import Message


class Node:
    def __init__(
        self,
        framework: Framework,
        nomssip_config: NomssipConfig,
        msg_handler: Callable[[NomssipMessage[Message]], Awaitable[None]],
    ):
        self.nomssip = Nomssip(
            framework,
            nomssip_config,
            msg_handler,
            noise_msg=NomssipMessage(NomssipMessage.Flag.NOISE, Message(-1, 0)),
        )

    def connect(
        self,
        peer: Node,
        inbound_conn: SimplexConnection[NomssipMessage[Message]],
        outbound_conn: SimplexConnection[NomssipMessage[Message]],
    ):
        connect_nodes(self.nomssip, peer.nomssip, inbound_conn, outbound_conn)

    async def send_message(self, msg: Message):
        """
        Send the message via Nomos Gossip to all connected peers.
        """
        await self.nomssip.publish(NomssipMessage(NomssipMessage.Flag.REAL, msg))
