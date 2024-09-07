import argparse
import os

import pandas as pd


def analyze(path: str) -> pd.DataFrame:
    medians = pd.read_csv(path).iloc[:, 1:].median(axis=1)
    return pd.DataFrame(
        {
            "min": medians.min(),
            "median": medians.median(),
            "mean": medians.mean(),
            "std": medians.std(),
            "max": medians.max(),
        },
        index=pd.Series([0]),
    )


def save_means_stats(means: list, outpath: str):
    series = pd.Series(means)
    means_stat = pd.DataFrame(
        {"mean": series.mean(), "std": series.std()}, index=pd.Series([0])
    )
    means_stat.to_csv(outpath, index=False)
    print(f"Saved {outpath}")


def analyze_all(path: str):
    means = []
    means_outpath = ""
    for root, dirs, files in os.walk(path):
        if os.path.basename(root).startswith("paramset_"):
            if len(means) > 0:
                assert means_outpath != ""
                save_means_stats(means, means_outpath)
            means = []
            means_outpath = os.path.join(root, "data_msg_count_means.csv")
        elif "data_msg_counts_stats.csv" in files:
            stats = pd.read_csv(os.path.join(root, "data_msg_counts_stats.csv"))
            means.append(stats["mean"].values[0])
        elif "data_msg_counts.csv" in files:
            stats = analyze(os.path.join(root, "data_msg_counts.csv"))
            means.append(stats["mean"].values[0])
            outpath = os.path.join(root, "data_msg_counts_stats.csv")
            stats.to_csv(outpath, index=False)
            print(f"Saved {outpath}")

    if len(means) > 0:
        assert means_outpath != ""
        save_means_stats(means, means_outpath)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Analyze message count data")
    parser.add_argument("path", type=str, help="dir path")
    args = parser.parse_args()
    analyze_all(args.path)
