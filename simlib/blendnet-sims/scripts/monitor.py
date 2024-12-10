import argparse
import json
from collections.abc import Iterable
from typing import Any

import pandas as pd

import mixlog


def analyze_monitors(input_stream: Iterable[str]) -> None:
    df = pd.DataFrame(monitor_records(input_stream))

    result = {
        "min": df["min"].min(),
        "avg": (df["num_conns"] * df["avg"]).sum() / df["num_conns"].sum(),
        "max": df["max"].max(),
        "std_min": df["std"].min(),
        "std_avg": df["std"].mean(),
        "std_max": df["std"].max(),
    }
    print(result)


def monitor_records(input_stream: Iterable[str]) -> list[Any]:
    records = []

    for line in input_stream:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue

        if "message_type" in record and "num_conns" in record:
            records.append(record)

    return records


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Mix connection monitor analysis")
    parser.add_argument(
        "--log-path",
        nargs="?",
        type=str,
        help="An input log file path. If not provided, input will be read from stdin.",
    )
    args = parser.parse_args()

    input = mixlog.get_input_stream(args.log_path)
    analyze_monitors(input)
