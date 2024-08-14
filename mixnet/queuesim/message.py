from dataclasses import dataclass

from framework.framework import Framework

MESSAGE_SIZE = 1


@dataclass
class Message:
    _id: int
    sent_time: float

    def id(self) -> int:
        return self._id

    def __len__(self) -> int:
        # Return any number here, since we don't use Sphinx encoding for queuesim and byte serialization.
        # This must be matched with NomssipConfig.msg_size.
        return MESSAGE_SIZE


class MessageBuilder:
    def __init__(self, framework: Framework):
        self.framework = framework
        self.next_id = 0

    def next(self) -> Message:
        msg = Message(self.next_id, self.framework.now())
        self.next_id += 1
        return msg
