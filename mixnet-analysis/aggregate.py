import argparse
import os

import pandas as pd


def aggregate(path: str):
    dataframes = []
    for root, dirs, files in os.walk(path):
        # print(f"root: {root}, dirs: {dirs}, files: {files}")
        if "paramset.csv" in files:
            df = pd.read_csv(os.path.join(root, "paramset.csv"))

            assert "data_msg_count_means.csv" in files
            mean_df = pd.read_csv(os.path.join(root, "data_msg_count_means.csv"))
            df["mean_data_msg_count"] = mean_df["mean"].values[0]

            assert "latency_stats.csv" in files
            latency_df = pd.read_csv(os.path.join(root, "latency_stats.csv"))
            df["latency_min"] = latency_df["min"].values[0]
            df["latency_median"] = latency_df["median"].values[0]
            df["latency_mean"] = latency_df["mean"].values[0]
            df["latency_std"] = latency_df["std"].values[0]
            df["latency_max"] = latency_df["max"].values[0]

            dataframes.append(df)
            print(f"Processed {root}")

    if dataframes:
        df = pd.concat(dataframes).sort_values(by=["paramset", "queue_type"])
        outpath = os.path.join(path, "aggregated.csv")
        df.to_csv(outpath, index=False)
        print(f"Saved {outpath}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Aggregate the results of all paramsets of an experiment"
    )
    parser.add_argument("path", type=str, help="dir path")
    args = parser.parse_args()
    aggregate(args.path)
