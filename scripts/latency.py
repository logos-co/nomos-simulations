# !/usr/bin/env python
import sys
from collections.abc import Iterable
from typing import Dict, Optional
import json_stream
import statistics
import argparse

from json_stream.base import TransientStreamingJSONObject

JsonStream = Iterable[TransientStreamingJSONObject]


class Record:
    def __hash__(self):
        return self.id

    def __init__(
            self,
            message_id: str,
            generator_node: Optional[str],
            generated_step: Optional[int],
            unwrapper_node: Optional[str],
            unwrapper_step: Optional[int]
    ):
        self.id = message_id
        self.generator_node = generator_node
        self.generated_step = int(generated_step) if generated_step is not None else None
        self.unwrapper_node = unwrapper_node
        self.unwrapper_step = int(unwrapper_step) if unwrapper_step is not None else None

    def __repr__(self):
        return f"[{self.id}] {self.generator_node}-{self.generated_step} -> {self.unwrapper_node}-{self.unwrapper_step}"

    @property
    def latency(self) -> Optional[int]:
        if self.unwrapper_step is not None and self.generated_step is not None:
            return self.unwrapper_step - self.generated_step


RecordStorage = Dict[str, Record]


def print_results(records: RecordStorage, step_duration: int):
    latencies = [message_record.latency for message_record in records.values()]
    valued_latencies = [latency for latency in latencies if latency is not None]
    incomplete_latencies = sum((1 for latency in latencies if latency is None))

    latency_average_steps = statistics.mean(valued_latencies)
    latency_median_steps = statistics.median(valued_latencies)

    print("[Results]")
    print(f"- Total messages: {len(latencies)}")
    print(f"    - Full latencies: {len(valued_latencies)}")
    print(f"    - Incomplete latencies: {incomplete_latencies}")
    print("- Average")
    print(f"    - Steps: {latency_average_steps}")
    print("    - Duration: {:.2f}ms".format(latency_average_steps * step_duration))
    print("- Median")
    print(f"    - Steps: {latency_median_steps}")
    print("    - Duration: {:.2f}ms".format(latency_median_steps * step_duration))


def parse_record_stream(stream: JsonStream) -> RecordStorage:
    storage: RecordStorage = {}

    for record in stream:
        node_id = record["node_id"]
        _step_id = record["step_id"]

        data_messages_generated = record["data_messages_generated"]
        for generated_message_id, generated_message_step_id in data_messages_generated.items():
            stored_message = storage.get(generated_message_id)
            if stored_message:
                stored_message.generator_node = node_id
                stored_message.generated_step = generated_message_step_id
            else:
                storage[generated_message_id] = Record(
                    generated_message_id, node_id, generated_message_step_id, None, None
                )

        data_messages_fully_unwrapped = record["data_messages_fully_unwrapped"]
        for generated_message_id, generated_message_step_id in data_messages_fully_unwrapped.items():
            stored_message = storage.get(generated_message_id)
            if stored_message:
                stored_message.unwrapper_node = node_id
                stored_message.unwrapper_step = generated_message_step_id
            else:
                storage[generated_message_id] = Record(
                    generated_message_id, None, None, node_id, generated_message_step_id
                )

    return storage


def from_pipe() -> JsonStream:
    yield from json_stream.load(sys.stdin)


def from_file(input_filename) -> JsonStream:
    with open(input_filename, "r") as file:
        data = json_stream.load(file)
        yield from data["records"]


def get_input_stream(input_filename: Optional[str]) -> JsonStream:
    if input_filename is not None:
        return from_file(input_filename)
    return from_pipe()


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
    parsed_records = parse_record_stream(input_stream)

    print_results(parsed_records, arguments.step_duration)
