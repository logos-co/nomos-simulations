from dataclasses import dataclass
from enum import Enum
from typing import Awaitable, Callable, Generic, Protocol, TypeVar, override

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


class HasIdAndLen(Protocol):
    def id(self) -> int: ...
    def __len__(self) -> int: ...


T = TypeVar("T", bound=HasIdAndLen)


class NomssipMessage(Generic[T]):
    class Flag(Enum):
        REAL = b"\x00"
        NOISE = b"\x01"

    def __init__(self, flag: Flag, message: T):
        self.flag = flag
        self.message = message

    def id(self) -> int:
        return self.message.id()

    def __len__(self) -> int:
        return len(self.flag.value) + len(self.message)


class Nomssip(Gossip[NomssipMessage[T]]):
    """
    A NomMix gossip channel that extends the Gossip channel
    by adding global transmission rate and noise generation.
    """

    def __init__(
        self,
        framework: Framework,
        config: NomssipConfig,
        handler: Callable[[NomssipMessage[T]], Awaitable[None]],
        noise_msg: NomssipMessage[T],
    ):
        super().__init__(framework, config, handler)
        self.config = config
        self.noise_msg = noise_msg

    @override
    def add_conn(
        self,
        inbound: SimplexConnection[NomssipMessage[T]],
        outbound: SimplexConnection[NomssipMessage[T]],
    ):
        super().add_conn(
            inbound,
            MixSimplexConnection[NomssipMessage[T]](
                self.framework,
                outbound,
                self.config.transmission_rate_per_sec,
                self.noise_msg,
                self.config.temporal_mix,
                self.config.skip_sending_noise,
            ),
        )

    @override
    async def _process_inbound_msg(
        self, msg: NomssipMessage[T], received_from: DuplexConnection
    ):
        match msg.flag:
            case NomssipMessage.Flag.NOISE:
                # Drop noise packet
                return
            case NomssipMessage.Flag.REAL:
                self.assert_message_size(msg.message)
                await super()._gossip(msg, [received_from])
                await self.handler(msg)

    @override
    async def publish(self, msg: NomssipMessage[T]):
        self.assert_message_size(msg.message)

        # Please see comments in super().publish() for the reason of the following line.
        if not self._check_update_cache(msg, publishing=True):
            await self._gossip(msg)
            await self.handler(msg)

    def assert_message_size(self, msg: T):
        # The message size must be fixed.
        assert len(msg) == self.config.msg_size, f"{len(msg)} != {self.config.msg_size}"
