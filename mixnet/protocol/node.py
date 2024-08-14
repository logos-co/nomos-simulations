from __future__ import annotations

from typing import Awaitable, Callable, Generic, Protocol, Self, Type, TypeVar

from pysphinx.sphinx import (
    ProcessedFinalHopPacket,
    ProcessedForwardHopPacket,
    SphinxPacket,
)

from framework import Framework
from protocol.config import GlobalConfig, NodeConfig
from protocol.connection import SimplexConnection
from protocol.error import PeeringDegreeReached
from protocol.gossip import Gossip
from protocol.nomssip import Nomssip, NomssipConfig, NomssipMessage
from protocol.sphinx import SphinxPacketBuilder


class HasIdAndLenAndBytes(Protocol):
    def id(self) -> int: ...
    def __len__(self) -> int: ...
    def __bytes__(self) -> bytes: ...
    @classmethod
    def from_bytes(cls, data: bytes) -> Self: ...


T = TypeVar("T", bound=HasIdAndLenAndBytes)


class Node(Generic[T]):
    """
    This represents any node in the network, which:
    - generates/gossips mix messages (Sphinx packets)
    - performs cryptographic mix (unwrapping Sphinx packets)
    - generates noise
    """

    def __init__(
        self,
        framework: Framework,
        config: NodeConfig,
        global_config: GlobalConfig,
        # A handler called when a node receives a broadcasted message originated from the last mix.
        broadcasted_msg_handler: Callable[[T], Awaitable[None]],
        # A handler called when a message is fully recovered by the last mix
        # and returns a new message to be broadcasted.
        recovered_msg_handler: Callable[[bytes], Awaitable[T]],
        noise_msg: T,
    ):
        self.framework = framework
        self.config = config
        self.global_config = global_config
        nomssip_config = NomssipConfig(
            config.gossip.peering_degree,
            global_config.transmission_rate_per_sec,
            SphinxPacketBuilder.size(global_config),
            config.temporal_mix,
        )
        self.nomssip = Nomssip(
            framework,
            nomssip_config,
            self.__process_msg,
            noise_msg=NomssipMessage[T](NomssipMessage.Flag.NOISE, noise_msg),
        )
        self.broadcast = Gossip[T](framework, config.gossip, broadcasted_msg_handler)
        self.recovered_msg_handler = recovered_msg_handler

    async def __process_msg(self, msg: NomssipMessage[T]) -> None:
        """
        A handler to process messages received via Nomssip channel
        """
        assert msg.flag == NomssipMessage.Flag.REAL

        sphinx_packet = SphinxPacket.from_bytes(
            bytes(msg.message), self.global_config.max_mix_path_length
        )
        result = await self.__process_sphinx_packet(sphinx_packet)
        match result:
            case SphinxPacket():
                # Gossip the next Sphinx packet
                t: Type[T] = type(msg.message)
                await self.nomssip.publish(
                    NomssipMessage[T](
                        NomssipMessage.Flag.REAL,
                        t.from_bytes(result.bytes()),
                    )
                )
            case bytes():
                # Broadcast the message fully recovered from Sphinx packets
                await self.broadcast.publish(await self.recovered_msg_handler(result))
            case None:
                return

    async def __process_sphinx_packet(
        self, packet: SphinxPacket
    ) -> SphinxPacket | bytes | None:
        """
        Unwrap the Sphinx packet and process the next Sphinx packet or the payload if possible
        """
        try:
            processed = packet.process(self.config.private_key)
            match processed:
                case ProcessedForwardHopPacket():
                    return processed.next_packet
                case ProcessedFinalHopPacket():
                    return processed.payload.recover_plain_playload()
        except ValueError:
            # Return nothing, if it cannot be unwrapped by the private key of this node.
            return None

    def connect_mix(
        self,
        peer: Node,
        inbound_conn: SimplexConnection[NomssipMessage[T]],
        outbound_conn: SimplexConnection[NomssipMessage[T]],
    ):
        connect_nodes(self.nomssip, peer.nomssip, inbound_conn, outbound_conn)

    def connect_broadcast(
        self,
        peer: Node,
        inbound_conn: SimplexConnection[T],
        outbound_conn: SimplexConnection[T],
    ):
        connect_nodes(self.broadcast, peer.broadcast, inbound_conn, outbound_conn)

    async def send_message(self, msg: T):
        """
        Build a Sphinx packet and gossip it to all connected peers.
        """
        # Here, we handle the case in which a msg is split into multiple Sphinx packets.
        # But, in practice, we expect a message to be small enough to fit in a single Sphinx packet.
        sphinx_packet, _ = SphinxPacketBuilder.build(
            bytes(msg),
            self.global_config,
            self.config.mix_path_length,
        )
        t: Type[T] = type(msg)
        await self.nomssip.publish(
            NomssipMessage(
                NomssipMessage.Flag.REAL, t.from_bytes(sphinx_packet.bytes())
            )
        )


def connect_nodes(
    self_channel: Gossip,
    peer_channel: Gossip,
    inbound_conn: SimplexConnection,
    outbound_conn: SimplexConnection,
):
    """
    Establish a duplex connection with a peer node.
    """
    if not self_channel.can_accept_conn() or not peer_channel.can_accept_conn():
        raise PeeringDegreeReached()

    # Register a duplex connection for its own use
    self_channel.add_conn(inbound_conn, outbound_conn)
    # Register a duplex connection for the peer
    peer_channel.add_conn(outbound_conn, inbound_conn)
