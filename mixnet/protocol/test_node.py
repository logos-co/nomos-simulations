import hashlib
from dataclasses import dataclass
from typing import Self
from unittest import IsolatedAsyncioTestCase

import framework.asyncio as asynciofw
from framework.framework import Queue
from protocol.connection import LocalSimplexConnection
from protocol.node import Node
from protocol.nomssip import NomssipMessage
from protocol.test_utils import (
    init_mixnet_config,
)


class TestNode(IsolatedAsyncioTestCase):
    async def test_node(self):
        framework = asynciofw.Framework()
        global_config, node_configs, _ = init_mixnet_config(10)

        queue: Queue[Message] = framework.queue()

        async def broadcasted_msg_handler(msg: Message) -> None:
            await queue.put(msg)

        async def recovered_msg_handler(msg: bytes) -> Message:
            return Message(msg)

        nodes = [
            Node[Message](
                framework,
                node_config,
                global_config,
                broadcasted_msg_handler,
                recovered_msg_handler,
                noise_msg=Message(b""),
            )
            for node_config in node_configs
        ]
        for i, node in enumerate(nodes):
            try:
                node.connect_mix(
                    nodes[(i + 1) % len(nodes)],
                    LocalSimplexConnection[NomssipMessage[Message]](framework),
                    LocalSimplexConnection[NomssipMessage[Message]](framework),
                )
                node.connect_broadcast(
                    nodes[(i + 1) % len(nodes)],
                    LocalSimplexConnection[Message](framework),
                    LocalSimplexConnection[Message](framework),
                )
            except ValueError as e:
                print(e)

        msg = Message(b"block selection")
        await nodes[0].send_message(msg)

        # Wait for all nodes to receive the broadcast
        num_nodes_received_broadcast = 0
        timeout = 15
        for _ in range(timeout):
            await framework.sleep(1)

            while not queue.empty():
                self.assertEqual(msg, await queue.get())
                num_nodes_received_broadcast += 1

            if num_nodes_received_broadcast == len(nodes):
                break

        self.assertEqual(len(nodes), num_nodes_received_broadcast)

    # TODO: check noise


@dataclass
class Message:
    data: bytes

    def id(self) -> int:
        return int.from_bytes(hashlib.sha256(self.data).digest(), byteorder="big")

    def __len__(self) -> int:
        return len(self.data)

    def __bytes__(self) -> bytes:
        return self.data

    @classmethod
    def from_bytes(cls, data: bytes) -> Self:
        return cls(data)
