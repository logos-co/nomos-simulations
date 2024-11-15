import sys
from collections.abc import Iterable
from typing import Optional


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
    stream = (
        get_file_stream(input_filename)
        if input_filename is not None
        else get_pipe_stream()
    )
    return line_to_json_stream(stream)
