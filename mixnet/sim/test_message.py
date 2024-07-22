import time
from unittest import TestCase

from sim.message import Message, UniqueMessageBuilder


class TestMessage(TestCase):
    def test_message_serde(self):
        msg = Message(time.time(), 10, b"hello")
        serialized = bytes(msg)
        deserialized = Message.from_bytes(serialized)
        self.assertEqual(msg, deserialized)


class TestUniqueMessageBuilder(TestCase):
    def test_uniqueness(self):
        builder = UniqueMessageBuilder()
        msg1 = builder.next(time.time(), b"hello")
        msg2 = builder.next(time.time(), b"hello")
        self.assertEqual(0, msg1.id)
        self.assertEqual(1, msg2.id)
        self.assertNotEqual(msg1, msg2)
        self.assertNotEqual(hash(msg1), hash(msg2))
