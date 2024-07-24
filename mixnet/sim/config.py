from __future__ import annotations

import hashlib
import random
from dataclasses import dataclass

import dacite
import yaml
from pysphinx.sphinx import X25519PrivateKey

from protocol.config import NodeConfig, NomssipConfig


@dataclass
class Config:
    simulation: SimulationConfig
    network: NetworkConfig
    logic: LogicConfig
    mix: MixConfig

    @classmethod
    def load(cls, yaml_path: str) -> Config:
        with open(yaml_path, "r") as f:
            data = yaml.safe_load(f)
        return dacite.from_dict(
            data_class=Config,
            data=data,
            config=dacite.Config(
                type_hooks={random.Random: seed_to_random}, strict=True
            ),
        )

    def node_configs(self) -> list[NodeConfig]:
        return [
            NodeConfig(
                self.__gen_private_key(i),
                self.mix.mix_path.random_length(),
                self.network.nomssip,
            )
            for i in range(self.network.num_nodes)
        ]

    def __gen_private_key(self, node_idx: int) -> X25519PrivateKey:
        return X25519PrivateKey.from_private_bytes(
            hashlib.sha256(node_idx.to_bytes(4, "big")).digest()[:32]
        )


@dataclass
class SimulationConfig:
    # Desired duration of the simulation in seconds
    # Since the simulation uses discrete time steps, the actual duration may be longer or shorter.
    duration_sec: int
    # Show all plots that have been drawn during the simulation
    show_plots: bool

    def __post_init__(self):
        assert self.duration_sec > 0


@dataclass
class NetworkConfig:
    # Total number of nodes in the entire network.
    num_nodes: int
    latency: LatencyConfig
    nomssip: NomssipConfig

    def __post_init__(self):
        assert self.num_nodes > 0


@dataclass
class LatencyConfig:
    # Minimum/maximum network latency between nodes in seconds.
    # A constant latency will be chosen randomly for each connection within the range [min_latency_sec, max_latency_sec].
    min_latency_sec: float
    max_latency_sec: float
    # Seed for the random number generator used to determine the network latencies.
    seed: random.Random

    def __post_init__(self):
        assert 0 <= self.min_latency_sec <= self.max_latency_sec
        assert self.seed is not None

    def random_latency(self) -> float:
        # round to milliseconds to make analysis not too heavy
        return round(self.seed.uniform(self.min_latency_sec, self.max_latency_sec), 3)


@dataclass
class MixConfig:
    # Global constant transmission rate of each connection in messages per second.
    transmission_rate_per_sec: int
    # Maximum size of a message in bytes that can be encapsulated in a single Sphinx packet.
    max_message_size: int
    mix_path: MixPathConfig

    def __post_init__(self):
        assert self.transmission_rate_per_sec > 0
        assert self.max_message_size > 0


@dataclass
class MixPathConfig:
    # Minimum number of mix nodes to be chosen for a Sphinx packet.
    min_length: int
    # Maximum number of mix nodes to be chosen for a Sphinx packet.
    max_length: int
    # Seed for the random number generator used to determine the mix path.
    seed: random.Random

    def __post_init__(self):
        assert 0 < self.min_length <= self.max_length
        assert self.seed is not None

    def random_length(self) -> int:
        return self.seed.randint(self.min_length, self.max_length)


@dataclass
class LogicConfig:
    sender_lottery: LotteryConfig


@dataclass
class LotteryConfig:
    # Interval between lottery draws in seconds.
    interval_sec: float
    # Probability of a node being selected as a sender in each lottery draw.
    probability: float
    # Seed for the random number generator used to determine the lottery winners.
    seed: random.Random

    def __post_init__(self):
        assert self.interval_sec > 0
        assert self.probability >= 0
        assert self.seed is not None


def seed_to_random(seed: int) -> random.Random:
    return random.Random(seed)
