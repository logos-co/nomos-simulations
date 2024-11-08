# !/usr/bin/env python
import sys
from collections.abc import Iterable
from typing import Dict, Optional
import json_stream
import statistics

from json_stream.base import TransientStreamingJSONObject

JsonStream = Iterable[TransientStreamingJSONObject]


class Record:
    id: str
    generator_node: Optional[str]
    generated_step: Optional[int]
    unwrapper_node: Optional[str]
    unwrapper_step: Optional[int]

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


def parse_record_stream(stream: JsonStream) -> RecordStorage:
    storage: RecordStorage = {}

    for record in stream:
        node_id = list(record["node_id"])
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


def get_input_stream(arguments) -> JsonStream:
    if len(arguments) == 0:
        # If no arguments are provided, assume pipe
        return from_pipe()
    elif len(arguments) == 1:
        # If more than argument is provided, assume the argument in pos 0 is the name of the file to parse
        return from_file(arguments[0])
    else:
        raise NotImplementedError(f"Unsupported number of arguments: {len(args)}")


if __name__ == "__main__":
    script, *args = sys.argv
    input_stream = get_input_stream(args)
    record_storage = parse_record_stream(input_stream)
    latencies = (message_record.latency for message_record in record_storage.values())
    print("[Average]")
    print("- Latency: ", statistics.mean(latencies))
