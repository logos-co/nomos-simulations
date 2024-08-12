import csv
import struct
from dataclasses import dataclass
from typing import Counter, Self

import pandas as pd
import usim

from framework.framework import Queue
from framework.usim import Framework
from protocol.connection import LocalSimplexConnection, SimplexConnection
from queuesim.config import Config
from queuesim.node import Node
from sim.connection import RemoteSimplexConnection
from sim.topology import build_full_random_topology


class Simulation:
    """
    Manages the entire cycle of simulation: initialization, running, and analysis.
    """

    def __init__(self, config: Config):
        self.config = config

    async def run(self, out_csv_path: str, topology_path: str):
        async with usim.Scope() as scope:
            self.framework = Framework(scope)
            self.message_builder = MessageBuilder(self.framework)
            await self.__run(out_csv_path, topology_path)
            self.framework.stop_tasks()

    async def __run(self, out_csv_path: str, topology_path: str):
        self.received_msg_queue: Queue[tuple[float, bytes]] = self.framework.queue()

        # Run and connect nodes
        nodes = self.__run_nodes()
        self.__connect_nodes(nodes, topology_path)

        # Choose and start senders
        senders = self.config.sender_generator.sample(nodes, k=self.config.num_senders)
        for sender in senders:
            self.framework.spawn(self.__run_sender(sender))

        # Open the output CSV file
        with open(out_csv_path, "w", newline="", buffering=8192) as f:
            # Use CSV writer which is less error-prone than manually writing rows to the file
            writer = csv.writer(f)
            writer.writerow(["dissemination_time", "sent_time", "all_received_time"])
            # To count how many nodes have received each message
            received_msg_counters: Counter[bytes] = Counter()
            # To count how many results (dissemination time) have been collected so far
            result_cnt = 0
            # Wait until all messages are disseminated to the entire network.
            while result_cnt < self.config.num_sent_msgs * self.config.num_senders:
                # Wait until a node notifies that it has received a new message.
                received_time, msg = await self.received_msg_queue.get()
                # If the message has been received by all nodes, calculate the dissemination time.
                received_msg_counters.update([msg])
                if received_msg_counters[msg] == len(nodes):
                    sent_time = Message.from_bytes(msg).sent_time
                    dissemination_time = received_time - sent_time
                    # Use repr to convert a float to a string with as much precision as Python can provide
                    writer.writerow(
                        [repr(dissemination_time), repr(sent_time), repr(received_time)]
                    )
                    result_cnt += 1

    def __run_nodes(self) -> list[Node]:
        return [
            Node(
                self.framework,
                self.config.nomssip,
                self.__process_msg,
            )
            for _ in range(self.config.num_nodes)
        ]

    async def __process_msg(self, msg: bytes) -> None:
        """
        A handler to process messages received via Nomos Gossip channel
        """
        # Notify that a new message has been received by the node.
        # The received time is also included in the notification.
        await self.received_msg_queue.put((self.framework.now(), msg))

    def __connect_nodes(self, nodes: list[Node], topology_path: str):
        topology = build_full_random_topology(
            rng=self.config.topology.seed,
            num_nodes=len(nodes),
            peering_degree=self.config.nomssip.peering_degree,
        )
        # Store the topology to a CSV file for later analysis
        pd.DataFrame(
            [(node, len(peers), list(peers)) for node, peers in topology.items()],
            columns=pd.Series(["node", "num_peers", "peers"]),
        ).to_csv(topology_path, index=False)
        # Sort the topology by node index for the connection RULE defined below.
        for node_idx, peer_indices in sorted(topology.items()):
            for peer_idx in peer_indices:
                # Since the topology is undirected, we only need to connect the two nodes once.
                # RULE: the node with the smaller index establishes the connection.
                assert node_idx != peer_idx
                if node_idx > peer_idx:
                    continue

                # Connect the node and peer for Nomos Gossip
                node = nodes[node_idx]
                peer = nodes[peer_idx]
                node.connect(peer, self.__create_conn(), self.__create_conn())

    def __create_conn(self) -> SimplexConnection:
        # If latency is always zero, use the local connection which is the lightest.
        if (
            self.config.latency.min_latency_sec
            == self.config.latency.max_latency_sec
            == 0
        ):
            return LocalSimplexConnection(self.framework)
        else:
            return RemoteSimplexConnection(
                self.config.latency,
                self.framework,
            )

    async def __run_sender(self, sender: Node):
        for i in range(self.config.num_sent_msgs):
            if i > 0:
                await self.framework.sleep(self.config.msg_interval_sec)
            msg = bytes(self.message_builder.next())
            await sender.send_message(msg)


@dataclass
class Message:
    id: int
    sent_time: float

    def __bytes__(self) -> bytes:
        return struct.pack("if", self.id, self.sent_time)

    @classmethod
    def from_bytes(cls, data: bytes) -> Self:
        id, sent_from = struct.unpack("if", data)
        return cls(id, sent_from)


class MessageBuilder:
    def __init__(self, framework: Framework):
        self.framework = framework
        self.next_id = 0

    def next(self) -> Message:
        msg = Message(self.next_id, self.framework.now())
        self.next_id += 1
        return msg
