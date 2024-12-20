# !/usr/bin/env python
import argparse
import json
import statistics
from collections.abc import Iterable
from typing import Dict, Optional

import mixlog


class Message:
    def __init__(self, message_id: str, step_a: Optional[int]):
        self.id = message_id
        self.step_a = int(step_a) if step_a is not None else None
        self.step_b = None

    def __hash__(self):
        return self.id

    def __repr__(self):
        return f"[{self.id}] {self.step_a} -> {self.step_b}"

    @property
    def latency(self) -> Optional[int]:
        if self.step_a is not None and self.step_b is not None:
            return abs(self.step_a - self.step_b)

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

    for _, record in filter(
        lambda x: x[0]
        in (
            "DataMessageGenerated",
            "CoverMessageGenerated",
            "MessageFullyUnwrapped",
        ),
        record_stream,
    ):
        payload_id = record["payload_id"]
        step_id = record["step_id"]

        if (stored_message := storage.get(payload_id)) is None:
            storage[payload_id] = Message(payload_id, step_id)
        else:
            stored_message.step_b = step_id

    return storage


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
