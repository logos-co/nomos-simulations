from collections import Counter, defaultdict

import matplotlib.pyplot as plt
import numpy
import pandas
from matplotlib.axes import Axes

from protocol.node import Node
from sim.connection import ObservedMeteredRemoteSimplexConnection
from sim.message import Message

# A map of nodes to their inbound/outbound connections
NodeConnectionsMap = dict[
    Node,
    tuple[
        list[ObservedMeteredRemoteSimplexConnection],
        list[ObservedMeteredRemoteSimplexConnection],
    ],
]


class ConnectionStats:
    def __init__(self):
        self.conns_per_node: NodeConnectionsMap = defaultdict(lambda: ([], []))

    def register(
        self,
        node: Node,
        inbound_conn: ObservedMeteredRemoteSimplexConnection,
        outbound_conn: ObservedMeteredRemoteSimplexConnection,
    ):
        self.conns_per_node[node][0].append(inbound_conn)
        self.conns_per_node[node][1].append(outbound_conn)

    def analyze(self):
        self.__message_sizes()
        self.__bandwidths_per_conn()
        self.__bandwidths_per_node()

    def __message_sizes(self):
        """
        Analyzes all message sizes sent across all connections of all nodes.
        """
        sizes: Counter[int] = Counter()
        for _, (_, outbound_conns) in self.conns_per_node.items():
            for conn in outbound_conns:
                sizes.update(conn.msg_sizes)

        df = pandas.DataFrame.from_dict(sizes, orient="index").reset_index()
        df.columns = ["msg_size", "count"]
        print("==========================================")
        print("Message Size Distribution")
        print("==========================================")
        print(f"{df}\n")

    def __bandwidths_per_conn(self):
        """
        Analyzes the bandwidth consumed by each simplex connection.
        """
        plt.plot(figsize=(12, 6))

        for _, (_, outbound_conns) in self.conns_per_node.items():
            for conn in outbound_conns:
                sending_bandwidths = conn.sending_bandwidths().map(lambda x: x / 1024)
                plt.plot(sending_bandwidths.index, sending_bandwidths)

        plt.title("Unidirectional Bandwidths per Connection")
        plt.xlabel("Time (s)")
        plt.ylabel("Bandwidth (KiB/s)")
        plt.ylim(bottom=0)
        plt.grid(True)
        plt.tight_layout()
        plt.draw()

    def __bandwidths_per_node(self):
        """
        Analyzes the inbound/outbound bandwidths consumed by each node (sum of all its connections).
        """
        _, axs = plt.subplots(nrows=2, ncols=1, figsize=(12, 6))
        assert isinstance(axs, numpy.ndarray)

        for i, (_, (inbound_conns, outbound_conns)) in enumerate(
            self.conns_per_node.items()
        ):
            inbound_bandwidths = (
                pandas.concat(
                    [conn.receiving_bandwidths() for conn in inbound_conns], axis=1
                )
                .sum(axis=1)
                .map(lambda x: x / 1024)
            )
            outbound_bandwidths = (
                pandas.concat(
                    [conn.sending_bandwidths() for conn in outbound_conns], axis=1
                )
                .sum(axis=1)
                .map(lambda x: x / 1024)
            )
            axs[0].plot(inbound_bandwidths.index, inbound_bandwidths, label=f"Node-{i}")
            axs[1].plot(
                outbound_bandwidths.index, outbound_bandwidths, label=f"Node-{i}"
            )

        axs[0].set_title("Inbound Bandwidths per Node")
        axs[0].set_xlabel("Time (s)")
        axs[0].set_ylabel("Bandwidth (KiB/s)")
        axs[0].legend()
        axs[0].set_ylim(bottom=0)
        axs[0].grid(True)

        axs[1].set_title("Outbound Bandwidths per Node")
        axs[1].set_xlabel("Time (s)")
        axs[1].set_ylabel("Bandwidth (KiB/s)")
        axs[1].legend()
        axs[1].set_ylim(bottom=0)
        axs[1].grid(True)

        plt.tight_layout()
        plt.draw()


class DisseminationTime:
    def __init__(self, num_nodes: int):
        # A collection of time taken for a message to propagate through all mix nodes in its mix route
        self.mix_propagation_times: list[float] = []
        # A collection of time taken for a message to be broadcasted from the last mix to all nodes in the network
        self.broadcast_dissemination_times: list[float] = []
        # Data structures to check if a message has been broadcasted to all nodes
        self.broadcast_status: Counter[Message] = Counter()
        self.num_nodes: int = num_nodes

    def add_mix_propagation_time(self, elapsed: float):
        self.mix_propagation_times.append(elapsed)

    def add_broadcasted_msg(self, msg: Message, elapsed: float):
        assert self.broadcast_status[msg] < self.num_nodes
        self.broadcast_status.update([msg])
        if self.broadcast_status[msg] == self.num_nodes:
            self.broadcast_dissemination_times.append(elapsed)

    def analyze(self):
        print("==========================================")
        print("Message Dissemination Time")
        print("==========================================")
        print("[Mix Propagation Times]")
        mix_propagation_times = pandas.Series(self.mix_propagation_times)
        print(mix_propagation_times.describe())
        print("")
        print("[Broadcast Dissemination Times]")
        broadcast_travel_times = pandas.Series(self.broadcast_dissemination_times)
        print(broadcast_travel_times.describe())
        print("")
