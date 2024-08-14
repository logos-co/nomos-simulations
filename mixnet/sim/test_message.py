import time
from unittest import TestCase

from sim.message import InnerMessage, UniqueInnerMessageBuilder


class TestMessage(TestCase):
    def test_inner_message_serde(self):
        msg = InnerMessage(time.time(), 10, b"hello")
        serialized = bytes(msg)
        deserialized = InnerMessage.from_bytes(serialized)
        self.assertEqual(msg, deserialized)


class TestUniqueInnerMessageBuilder(TestCase):
    def test_uniqueness(self):
        builder = UniqueInnerMessageBuilder()
        msg1 = builder.next(time.time(), b"hello")
        msg2 = builder.next(time.time(), b"hello")
        self.assertEqual(0, msg1.id)
        self.assertEqual(1, msg2.id)
        self.assertNotEqual(msg1, msg2)
