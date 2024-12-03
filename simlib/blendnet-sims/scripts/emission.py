import argparse
import json
from collections.abc import Iterable
from typing import Any

import matplotlib
import matplotlib.pyplot as plt
import pandas as pd

import mixlog


def plot_emissions(input_stream: Iterable[str], plot_path: str) -> None:
    df = pd.DataFrame(emission_records(input_stream))

    plt.figure(figsize=(12, 6))
    plt.scatter(df["step_id"], df["node_id"], c="red", marker="x", alpha=0.6)
    plt.xlabel("Step ID")
    plt.ylabel("Node ID")
    plt.title("Distribution of Emissions")
    plt.tight_layout()
    plt.savefig(plot_path)
    if matplotlib.is_interactive():
        plt.show()


def emission_records(input_stream: Iterable[str]) -> list[Any]:
    records = []

    for line in input_stream:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue

        if "emission_type" in record:
            records.append(record)

    return records


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Mix emission analysis")
    parser.add_argument(
        "--log-path",
        nargs="?",
        type=str,
        help="An input log file path. If not provided, input will be read from stdin.",
    )
    parser.add_argument(
        "--plot-png-path", required=True, type=str, help="An output plot PNG file path"
    )
    args = parser.parse_args()

    input = mixlog.get_input_stream(args.log_path)
    plot_emissions(input, args.plot_png_path)
