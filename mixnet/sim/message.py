import pickle
from dataclasses import dataclass
from typing import Self


@dataclass
class Message:
    """
    A message structure for simulation, which will be sent through mix nodes
    and eventually broadcasted to all nodes in the network.

    The `id` must ensure the uniqueness of the message.
    """

    created_at: float
    id: int
    body: bytes

    def __bytes__(self):
        return pickle.dumps(self)

    @classmethod
    def from_bytes(cls, data: bytes) -> Self:
        return pickle.loads(data)

    def __hash__(self) -> int:
        return self.id


class UniqueMessageBuilder:
    """
    Builds a unique message with an incremental ID,
    assuming that the simulation is run in a single thread.
    """

    def __init__(self):
        self.next_id = 0

    def next(self, created_at: float, body: bytes) -> Message:
        msg = Message(created_at, self.next_id, body)
        self.next_id += 1
        return msg
