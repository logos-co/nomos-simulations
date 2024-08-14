from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Awaitable, Callable, Self, override

from framework import Framework
from protocol.connection import (
    DuplexConnection,
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
    async def _process_inbound_msg(self, msg: bytes, received_from: DuplexConnection):
        packet = FlaggedPacket.from_bytes(msg)
        match packet.flag:
            case FlaggedPacket.Flag.NOISE:
                # Drop noise packet
                return
            case FlaggedPacket.Flag.REAL:
                self.assert_message_size(packet.message)
                await super()._gossip(msg, [received_from])
                await self.handler(packet.message)

    @override
    async def publish(self, msg: bytes):
        self.assert_message_size(msg)

        packet = FlaggedPacket(FlaggedPacket.Flag.REAL, msg).bytes()
        # Please see comments in super().publish() for the reason of the following line.
        if not self._check_update_cache(packet, publishing=True):
            await self._gossip(packet)
            await self.handler(msg)

    def assert_message_size(self, msg: bytes):
        # The message size must be fixed.
        assert len(msg) == self.config.msg_size, f"{len(msg)} != {self.config.msg_size}"


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
