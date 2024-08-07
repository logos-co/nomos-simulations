from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Awaitable, Callable, Self, override

from framework import Framework
from protocol.connection import (
    MixSimplexConnection,
    SimplexConnection,
)
from protocol.gossip import Gossip, GossipConfig
from protocol.temporalmix import TemporalMixConfig


@dataclass
class NomssipConfig(GossipConfig):
    transmission_rate_per_sec: int
    msg_size: int
    temporal_mix: TemporalMixConfig
    # OPTIMIZATION ONLY FOR EXPERIMENTS WITHOUT BANDWIDTH MEASUREMENT
    # If True, skip sending a noise even if it's time to send one.
    skip_sending_noise: bool = False


class Nomssip(Gossip):
    """
    A NomMix gossip channel that extends the Gossip channel
    by adding global transmission rate and noise generation.
    """

    def __init__(
        self,
        framework: Framework,
        config: NomssipConfig,
        handler: Callable[[bytes], Awaitable[None]],
    ):
        super().__init__(framework, config, handler)
        self.config = config

    @override
    def add_conn(self, inbound: SimplexConnection, outbound: SimplexConnection):
        noise_packet = FlaggedPacket(
            FlaggedPacket.Flag.NOISE, bytes(self.config.msg_size)
        ).bytes()
        super().add_conn(
            inbound,
            MixSimplexConnection(
                self.framework,
                outbound,
                self.config.transmission_rate_per_sec,
                noise_packet,
                self.config.temporal_mix,
                self.config.skip_sending_noise,
            ),
        )

    @override
    async def process_inbound_msg(self, msg: bytes):
        packet = FlaggedPacket.from_bytes(msg)
        match packet.flag:
            case FlaggedPacket.Flag.NOISE:
                # Drop noise packet
                return
            case FlaggedPacket.Flag.REAL:
                await self.__gossip_flagged_packet(packet)
                await self.handler(packet.message)

    @override
    async def gossip(self, msg: bytes):
        """
        Gossip a message to all connected peers with prepending a message flag
        """
        # The message size must be fixed.
        assert len(msg) == self.config.msg_size, f"{len(msg)} != {self.config.msg_size}"

        packet = FlaggedPacket(FlaggedPacket.Flag.REAL, msg)
        await self.__gossip_flagged_packet(packet)

    async def __gossip_flagged_packet(self, packet: FlaggedPacket):
        """
        An internal method to send a flagged packet to all connected peers
        """
        await super().gossip(packet.bytes())


class FlaggedPacket:
    class Flag(Enum):
        REAL = b"\x00"
        NOISE = b"\x01"

    def __init__(self, flag: Flag, message: bytes):
        self.flag = flag
        self.message = message

    def bytes(self) -> bytes:
        return self.flag.value + self.message

    @classmethod
    def from_bytes(cls, packet: bytes) -> Self:
        """
        Parse a flagged packet from bytes
        """
        if len(packet) < 1:
            raise ValueError("Invalid message format")
        return cls(cls.Flag(packet[:1]), packet[1:])
