# !/usr/bin/env python
import json
import sys
from collections.abc import Iterable
from typing import Dict, Optional
import statistics
import argparse

from json_stream.base import TransientStreamingJSONObject

JsonStream = Iterable[TransientStreamingJSONObject]


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


MessageStorage = Dict[str, Message]


def compute_results(message_storage: MessageStorage, step_duration: int) -> str:
    latencies = [message_record.latency for message_record in message_storage.values()]
    valued_latencies = [latency for latency in latencies if latency is not None]
    incomplete_latencies = sum((1 for latency in latencies if latency is None))

    total_messages = len(latencies)
    total_messages_full_latency = len(valued_latencies)
    total_messages_incomplete_latency = incomplete_latencies
    latency_average_steps = statistics.mean(valued_latencies)
    latency_average_ms = "{:.2f}ms".format(latency_average_steps * step_duration)
    latency_median_steps = statistics.median(valued_latencies)
    latency_median_ms = "{:.2f}ms".format(latency_median_steps * step_duration)
    max_latency_steps = max(valued_latencies)
    max_latency_ms = "{:.2f}ms".format(max_latency_steps * step_duration)
    min_latency_steps = min(valued_latencies)
    min_latency_ms = "{:.2f}ms".format(min_latency_steps * step_duration)

    return f"""[Results]
- Total messages: {total_messages}
    - Full latencies: {total_messages_full_latency}
    - Incomplete latencies: {total_messages_incomplete_latency}
- Averages
    - Steps: {latency_average_steps}
    - Duration: {latency_average_ms}
- Median
    - Steps: {latency_median_steps}
    - Duration: {latency_median_ms}
- Max
    - Steps: {max_latency_steps}
    - Duration: {max_latency_ms}
- Min
    - Steps: {min_latency_steps}
    - Duration: {min_latency_ms}"""


def parse_record_stream(record_stream: Iterable[str]) -> MessageStorage:
    storage: MessageStorage = {}

    for record in record_stream:
        json_record = json.loads(record)
        payload_id = json_record["payload_id"]
        step_id = json_record["step_id"]

        if (stored_message := storage.get(payload_id)) is None:
            storage[payload_id] = Message(payload_id, step_id)
        else:
            stored_message.step_b = step_id

    return storage


def line_to_json_stream(record_stream: Iterable[str]) -> Iterable[str]:
    for record in record_stream:
        bracket_pos = record.rfind("{")
        yield record[bracket_pos:]


def get_pipe_stream() -> Iterable[str]:
    yield from sys.stdin


def get_file_stream(input_filename) -> Iterable[str]:
    with open(input_filename, "r") as file:
        yield from file


def get_input_stream(input_filename: Optional[str]) -> Iterable[str]:
    stream = get_file_stream(input_filename) if input_filename is not None else get_pipe_stream()
    return line_to_json_stream(stream)


def build_argument_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Log analysis for nomos-simulations.")
    parser.add_argument(
        "--step-duration",
        type=int,
        default=100,
        help="Duration (in ms) of each step in the simulation."
    )
    parser.add_argument(
        "input_file",
        nargs="?",
        help="The file to parse. If not provided, input will be read from stdin."
    )
    return parser


if __name__ == "__main__":
    argument_parser = build_argument_parser()
    arguments = argument_parser.parse_args()

    input_stream = get_input_stream(arguments.input_file)
    messages = parse_record_stream(input_stream)

    results = compute_results(messages, arguments.step_duration)
    print(results)
