# !/usr/bin/env python
import argparse
import json
import statistics
from collections.abc import Iterable
from typing import Dict, Optional

import mixlog


class Event:
    def __init__(self, step_id: int, node_id: int):
        self.step_id = step_id
        self.node_id = node_id


class Latency:
    def __init__(self, start_event: Event):
        self.start_event = start_event
        self.end_event = None

    def finish(self, event: Event):
        assert self.end_event is None
        assert event.step_id >= self.start_event.step_id
        self.end_event = event

    def finished(self) -> bool:
        return self.end_event is not None

    @property
    def value(self) -> Optional[int]:
        if self.end_event is None:
            return None
        return self.end_event.step_id - self.start_event.step_id


class Message:
    def __init__(self, message_id: str, msg_gen_event: Event):
        self.id = message_id
        self.total_latency = Latency(msg_gen_event)
        self.persistent_transmission_latencies: list[Latency] = []
        self.blend_latencies: list[Latency] = []

    def __hash__(self):
        return self.id

    def fully_unwrapped(self, event: Event):
        self.total_latency.finish(event)

    def persistent_transmission_scheduled(self, event: Event):
        Message.start_new_latency(self.persistent_transmission_latencies, event)

    def persistent_transmission_released(self, event: Event):
        Message.finish_recent_latency(self.persistent_transmission_latencies, event)

    def blend_scheduled(self, event: Event):
        Message.start_new_latency(self.blend_latencies, event)

    def blend_released(self, event: Event):
        Message.finish_recent_latency(self.blend_latencies, event)

    @staticmethod
    def start_new_latency(latencies: list[Latency], event: Event):
        latencies.append(Latency(event))

    @staticmethod
    def finish_recent_latency(latencies: list[Latency], event: Event):
        for latency in reversed(latencies):
            if latency.start_event.node_id == event.node_id:
                assert not latency.finished()
                latency.finish(event)
                return
        raise Exception("No latency to finish")

    @property
    def latency(self) -> Optional[int]:
        return self.total_latency.value

    def __eq__(self, other):
        if not isinstance(other, Message):
            return NotImplemented
        return self.latency == other.latency

    def __lt__(self, other):
        if not isinstance(other, Message):
            return NotImplemented
        if self.latency is None or other.latency is None:
            return NotImplemented
        return self.latency < other.latency


MessageStorage = Dict[str, Message]


def compute_results(
    message_storage: MessageStorage, step_duration: int
) -> dict[str, int | float | str]:
    complete_messages = [
        message for message in message_storage.values() if message.latency is not None
    ]
    incomplete_messages = sum(
        (1 for message in message_storage.values() if message.latency is None)
    )

    total_messages = len(message_storage)
    total_complete_messages = len(complete_messages)
    total_incomplete_messages = incomplete_messages

    complete_latencies = [
        message.latency for message in complete_messages if message.latency is not None
    ]
    latency_average_steps = statistics.mean(complete_latencies)
    latency_average_ms = "{:.2f}".format(latency_average_steps * step_duration)
    latency_median_steps = statistics.median(complete_latencies)
    latency_median_ms = "{:.2f}".format(latency_median_steps * step_duration)

    max_message = max(complete_messages)
    max_latency_steps = max_message.latency
    assert max_latency_steps is not None
    max_latency_ms = "{:.2f}".format(max_latency_steps * step_duration)

    min_message = min(complete_messages)
    min_latency_steps = min_message.latency
    assert min_latency_steps is not None
    min_latency_ms = "{:.2f}".format(min_latency_steps * step_duration)

    return {
        "total_messages": total_messages,
        "total_complete_messages": total_complete_messages,
        "total_incomplete_messages": total_incomplete_messages,
        "latency_average_steps": latency_average_steps,
        "latency_average_ms": latency_average_ms,
        "latency_median_steps": latency_median_steps,
        "latency_median_ms": latency_median_ms,
        "max_latency_message_id": max_message.id,
        "max_latency_steps": max_latency_steps,
        "max_latency_ms": max_latency_ms,
        "min_latency_message_id": min_message.id,
        "min_latency_steps": min_latency_steps,
        "min_latency_ms": min_latency_ms,
    }


def parse_record_stream(record_stream: Iterable[tuple[str, dict]]) -> MessageStorage:
    storage: MessageStorage = {}

    for topic, record in record_stream:
        if topic in ("DataMessageGenerated", "CoverMessageGenerated"):
            payload_id = record["payload_id"]
            storage[payload_id] = Message(payload_id, event_from_record(record))
        elif topic == "MessageFullyUnwrapped":
            storage[record["payload_id"]].fully_unwrapped(event_from_record(record))
        elif topic == "PersistentTransmissionScheduled":
            storage[record["payload_id"]].persistent_transmission_scheduled(
                event_from_record(record)
            )
        elif topic == "PersistentTransmissionReleased":
            storage[record["payload_id"]].persistent_transmission_released(
                event_from_record(record)
            )
        elif topic == "BlendScheduled":
            storage[record["payload_id"]].blend_scheduled(event_from_record(record))
        elif topic == "BlendReleased":
            storage[record["payload_id"]].blend_released(event_from_record(record))

    return storage


def event_from_record(record: dict) -> Event:
    return Event(record["step_id"], record["node_id"])


def build_argument_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Log analysis for nomos-simulations.")
    parser.add_argument(
        "--step-duration",
        type=int,
        default=100,
        help="Duration (in ms) of each step in the simulation.",
    )
    parser.add_argument(
        "input_file",
        nargs="?",
        help="The file to parse. If not provided, input will be read from stdin.",
    )
    return parser


if __name__ == "__main__":
    argument_parser = build_argument_parser()
    arguments = argument_parser.parse_args()

    input_stream = mixlog.get_input_stream(arguments.input_file)
    messages = parse_record_stream(input_stream)

    results = compute_results(messages, arguments.step_duration)
    print(json.dumps(results, indent=4))
