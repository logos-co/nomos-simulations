from pprint import pprint

import usim
from matplotlib import pyplot

import framework.usim as usimfw
from protocol.config import GlobalConfig, MixMembership, NodeInfo
from protocol.node import Node
from protocol.nomssip import NomssipMessage
from protocol.sphinx import SphinxPacketBuilder
from sim.config import Config
from sim.connection import (
    MeteredRemoteSimplexConnection,
    ObservedMeteredRemoteSimplexConnection,
)
from sim.message import InnerMessage, Message, UniqueInnerMessageBuilder
from sim.state import NodeState, NodeStateTable
from sim.stats import ConnectionStats, DisseminationTime
from sim.topology import build_full_random_topology


class Simulation:
    """
    Manages the entire cycle of simulation: initialization, running, and analysis.
    """

    def __init__(self, config: Config):
        self.config = config
        self.inner_msg_builder = UniqueInnerMessageBuilder()
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
            nodes = self.__init_nodes()
            self.__connect_nodes(nodes, node_state_table, conn_stats)
            for i, node in enumerate(nodes):
                print(f"Spawning node-{i} with {len(node.nomssip.conns)} conns")
                self.framework.spawn(self.__run_node_logic(node))

        # Return analysis tools once the μSim scope is done
        return conn_stats, node_state_table

    def __init_nodes(self) -> list[Node[Message]]:
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

        # Initialize/return Node instances
        noise_msg = Message(bytes(SphinxPacketBuilder.size(global_config)))
        return [
            Node[Message](
                self.framework,
                node_config,
                global_config,
                self.__process_broadcasted_msg,
                self.__process_recovered_msg,
                noise_msg,
            )
            for node_config in node_configs
        ]

    def __connect_nodes(
        self,
        nodes: list[Node[Message]],
        node_state_table: NodeStateTable,
        conn_stats: ConnectionStats,
    ):
        topology = build_full_random_topology(
            self.config.network.topology.seed,
            len(nodes),
            self.config.network.gossip.peering_degree,
        )
        print("Topology:")
        pprint(topology)

        meter_start_time = self.framework.now()
        # Sort the topology by node index for the connection RULE defined below.
        for node_idx, peer_indices in sorted(topology.items()):
            for peer_idx in peer_indices:
                # Since the topology is undirected, we only need to connect the two nodes once.
                # RULE: the node with the smaller index establishes the connection.
                assert node_idx != peer_idx
                if node_idx > peer_idx:
                    continue

                node = nodes[node_idx]
                peer = nodes[peer_idx]
                node_states = node_state_table[node_idx]
                peer_states = node_state_table[peer_idx]

                # Connect the node and peer for Nomos Gossip
                inbound_conn, outbound_conn = (
                    self.__create_observed_conn(
                        meter_start_time, peer_states, node_states
                    ),
                    self.__create_observed_conn(
                        meter_start_time, node_states, peer_states
                    ),
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

    def __create_observed_conn(
        self,
        meter_start_time: float,
        sender_states: list[NodeState],
        receiver_states: list[NodeState],
    ) -> ObservedMeteredRemoteSimplexConnection[NomssipMessage[Message]]:
        return ObservedMeteredRemoteSimplexConnection[NomssipMessage[Message]](
            self.config.network.latency,
            self.framework,
            meter_start_time,
            sender_states,
            receiver_states,
        )

    def __create_conn(
        self,
        meter_start_time: float,
    ) -> MeteredRemoteSimplexConnection[Message]:
        return MeteredRemoteSimplexConnection[Message](
            self.config.network.latency,
            self.framework,
            meter_start_time,
        )

    async def __run_node_logic(self, node: Node[Message]):
        """
        Runs the lottery periodically to check if the node is selected to send a block.
        If the node is selected, creates a block and sends it through mix nodes.
        """
        lottery_config = self.config.logic.sender_lottery
        while True:
            await self.framework.sleep(lottery_config.interval_sec)
            if lottery_config.seed.random() < lottery_config.probability:
                inner_msg = self.inner_msg_builder.next(
                    self.framework.now(), b"selected block"
                )
                await node.send_message(Message(bytes(inner_msg)))

    async def __process_broadcasted_msg(self, msg: Message):
        """
        Process a broadcasted message originated from the last mix.
        """
        inner_msg = InnerMessage.from_bytes(msg.data)
        elapsed = self.framework.now() - inner_msg.created_at
        self.dissemination_time.add_broadcasted_msg(msg, elapsed)

    async def __process_recovered_msg(self, msg: bytes) -> Message:
        """
        Process a message fully recovered by the last mix
        and returns a new message to be broadcasted.
        """
        inner_msg = InnerMessage.from_bytes(Message.from_bytes(msg).data)
        elapsed = self.framework.now() - inner_msg.created_at
        self.dissemination_time.add_mix_propagation_time(elapsed)

        # Update the timestamp and return the message to be broadcasted,
        # so that the broadcast dissemination time can be calculated from now.
        inner_msg.created_at = self.framework.now()
        return Message(bytes(inner_msg))
