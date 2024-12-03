from unittest import IsolatedAsyncioTestCase

import framework.asyncio as asynciofw
from framework.framework import Queue
from protocol.connection import LocalSimplexConnection
from protocol.node import Node
from protocol.test_utils import (
    init_mixnet_config,
)


class TestNode(IsolatedAsyncioTestCase):
    async def test_node(self):
        framework = asynciofw.Framework()
        global_config, node_configs, _ = init_mixnet_config(10)

        queue: Queue[bytes] = framework.queue()

        async def broadcasted_msg_handler(msg: bytes) -> None:
            await queue.put(msg)

        nodes = [
            Node(framework, node_config, global_config, broadcasted_msg_handler)
            for node_config in node_configs
        ]
        for i, node in enumerate(nodes):
            try:
                node.connect_mix(
                    nodes[(i + 1) % len(nodes)],
                    LocalSimplexConnection(framework),
                    LocalSimplexConnection(framework),
                )
                node.connect_broadcast(
                    nodes[(i + 1) % len(nodes)],
                    LocalSimplexConnection(framework),
                    LocalSimplexConnection(framework),
                )
            except ValueError as e:
                print(e)

        await nodes[0].send_message(b"block selection")

        # Wait for all nodes to receive the broadcast
        num_nodes_received_broadcast = 0
        timeout = 15
        for _ in range(timeout):
            await framework.sleep(1)

            while not queue.empty():
                self.assertEqual(b"block selection", await queue.get())
                num_nodes_received_broadcast += 1

            if num_nodes_received_broadcast == len(nodes):
                break

        self.assertEqual(len(nodes), num_nodes_received_broadcast)

    # TODO: check noise
