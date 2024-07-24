import random
from dataclasses import dataclass

from protocol.nomssip import NomssipConfig
from sim.config import LatencyConfig, TopologyConfig


@dataclass
class Config:
    num_nodes: int
    nomssip: NomssipConfig
    topology: TopologyConfig
    latency: LatencyConfig
    num_sent_msgs: int
    msg_interval_sec: float
    num_senders: int
    sender_generator: random.Random
