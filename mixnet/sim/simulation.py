from dataclasses import asdict, dataclass
from typing import Self

import usim
from matplotlib import pyplot

import framework.usim as usimfw
from framework import Framework
from protocol.config import GlobalConfig, MixMembership, NodeInfo
from protocol.node import Node, PeeringDegreeReached
from sim.config import Config
from sim.connection import (
    MeteredRemoteSimplexConnection,
    ObservedMeteredRemoteSimplexConnection,
)
from sim.message import Message, UniqueMessageBuilder
from sim.state import NodeState, NodeStateTable
from sim.stats import ConnectionStats, DisseminationTime


class Simulation:
    """
    Manages the entire cycle of simulation: initialization, running, and analysis.
    """

    def __init__(self, config: Config):
        self.config = config
        self.msg_builder = UniqueMessageBuilder()
        self.dissemination_time = DisseminationTime(self.config.network.num_nodes)

    async def run(self):
        # Run the simulation
        conn_stats, node_state_table = await self.__run()
        # Analyze the dissemination times
        self.dissemination_time.analyze()
        # Analyze the simulation results
        conn_stats.analyze()
        node_state_table.analyze()
        # Show plots
        if self.config.simulation.show_plots:
            pyplot.show()

    async def __run(self) -> tuple[ConnectionStats, NodeStateTable]:
        # Initialize analysis tools
        node_state_table = NodeStateTable(
            self.config.network.num_nodes, self.config.simulation.duration_sec
        )
        conn_stats = ConnectionStats()

        # Create a μSim scope and run the simulation
        async with usim.until(usim.time + self.config.simulation.duration_sec) as scope:
            self.framework = usimfw.Framework(scope)
            nodes = self.__init_nodes(node_state_table, conn_stats)
            for node in nodes:
                self.framework.spawn(self.__run_node_logic(node))

        # Return analysis tools once the μSim scope is done
        return conn_stats, node_state_table

    def __init_nodes(
        self, node_state_table: NodeStateTable, conn_stats: ConnectionStats
    ) -> list[Node]:
        # Initialize node/global configurations
        node_configs = self.config.node_configs()
        global_config = GlobalConfig(
            MixMembership(
                [
                    NodeInfo(node_config.private_key.public_key())
                    for node_config in node_configs
                ],
                self.config.mix.mix_path.seed,
            ),
            self.config.mix.transmission_rate_per_sec,
            self.config.mix.max_message_size,
            self.config.mix.mix_path.max_length,
        )

        # Initialize Node instances
        nodes = [
            Node(
                self.framework,
                node_config,
                global_config,
                self.__process_broadcasted_msg,
                self.__process_recovered_msg,
            )
            for node_config in node_configs
        ]

        # Connect nodes to each other
        meter_start_time = self.framework.now()
        for i, node in enumerate(nodes):
            # For now, we only consider a simple ring topology for simplicity.
            # TODO: Use a more realistic random (deterministic) topology.
            peer_idx = (i + 1) % len(nodes)
            peer = nodes[peer_idx]
            node_states = node_state_table[i]
            peer_states = node_state_table[peer_idx]

            # Connect the node and peer for Nomssip
            inbound_conn, outbound_conn = (
                self.__create_observed_conn(meter_start_time, peer_states, node_states),
                self.__create_observed_conn(meter_start_time, node_states, peer_states),
            )
            node.connect_mix(peer, inbound_conn, outbound_conn)
            # Register the connections to the connection statistics
            conn_stats.register(node, inbound_conn, outbound_conn)
            conn_stats.register(peer, outbound_conn, inbound_conn)

            # Connect the node and peer for broadcasting.
            node.connect_broadcast(
                peer,
                self.__create_conn(meter_start_time),
                self.__create_conn(meter_start_time),
            )

        return nodes

    def __create_observed_conn(
        self,
        meter_start_time: float,
        sender_states: list[NodeState],
        receiver_states: list[NodeState],
    ) -> ObservedMeteredRemoteSimplexConnection:
        return ObservedMeteredRemoteSimplexConnection(
            self.config.network.latency,
            self.framework,
            meter_start_time,
            sender_states,
            receiver_states,
        )

    def __create_conn(
        self,
        meter_start_time: float,
    ) -> MeteredRemoteSimplexConnection:
        return MeteredRemoteSimplexConnection(
            self.config.network.latency,
            self.framework,
            meter_start_time,
        )

    async def __run_node_logic(self, node: Node):
        """
        Runs the lottery periodically to check if the node is selected to send a block.
        If the node is selected, creates a block and sends it through mix nodes.
        """
        lottery_config = self.config.logic.sender_lottery
        while True:
            await self.framework.sleep(lottery_config.interval_sec)
            if lottery_config.seed.random() < lottery_config.probability:
                msg = self.msg_builder.next(self.framework.now(), b"selected block")
                await node.send_message(bytes(msg))

    async def __process_broadcasted_msg(self, msg: bytes):
        """
        Process a broadcasted message originated from the last mix.
        """
        message = Message.from_bytes(msg)
        elapsed = self.framework.now() - message.created_at
        self.dissemination_time.add_broadcasted_msg(message, elapsed)

    async def __process_recovered_msg(self, msg: bytes) -> bytes:
        """
        Process a message fully recovered by the last mix
        and returns a new message to be broadcasted.
        """
        message = Message.from_bytes(msg)
        elapsed = self.framework.now() - message.created_at
        self.dissemination_time.add_mix_propagation_time(elapsed)

        # Update the timestamp and return the message to be broadcasted,
        # so that the broadcast dissemination time can be calculated from now.
        message.created_at = self.framework.now()
        return bytes(message)
