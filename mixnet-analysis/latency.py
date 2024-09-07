import argparse
import os

import pandas as pd


def save_latency_stats(latencies: list, outpath: str):
    series = pd.Series(latencies)
    means_stat = pd.DataFrame(
        {
            "min": series.min(),
            "median": series.median(),
            "mean": series.mean(),
            "std": series.std(),
            "max": series.max(),
        },
        index=pd.Series([0]),
    )
    means_stat.to_csv(outpath, index=False)
    print(f"Saved {outpath}")


def aggregate(path: str):
    latencies = []
    latencies_outpath = ""
    for root, dirs, files in os.walk(path):
        if os.path.basename(root).startswith("paramset_"):
            if len(latencies) > 0:
                assert latencies_outpath != ""
                save_latency_stats(latencies, latencies_outpath)
            latencies = []
            latencies_outpath = os.path.join(root, "latency_stats.csv")
        elif "latency.csv" in files:
            df = pd.read_csv(os.path.join(root, "latency.csv"))
            latencies.extend(df["latency"].to_list())

    if len(latencies) > 0:
        assert latencies_outpath != ""
        save_latency_stats(latencies, latencies_outpath)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Aggregate latencies")
    parser.add_argument("path", type=str, help="dir path")
    args = parser.parse_args()
    aggregate(args.path)
