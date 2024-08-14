import glob
import os
import re

import pandas as pd

from queuesim.queuesim import PARAMSET_INFO_COLUMNS

RESULT_COLUMNS = PARAMSET_INFO_COLUMNS + [
    "dtime_count",
    "dtime_mean",
    "dtime_std",
    "dtime_min",
    "dtime_25%",
    "dtime_50%",
    "dtime_75%",
    "dtime_max",
]


def calculate_session_stats(dir: str):
    session_result_path = f"{dir}/session.csv"
    assert not os.path.exists(session_result_path)
    pd.DataFrame(columns=pd.Series(RESULT_COLUMNS)).to_csv(
        session_result_path, index=False
    )
    print(f"Initialized a CSV file: {session_result_path}")

    paramset_dirs = [
        path for path in glob.glob(f"{dir}/paramset_*") if os.path.isdir(path)
    ]
    for paramset_dir in paramset_dirs:
        __calculate_paramset_stats(paramset_dir, session_result_path)
        print(f"Appended a row to {session_result_path}")

    # Load the completed session CSV file, sort rows by paramset_id,
    # and overrite the file with the sorted rows.
    pd.read_csv(session_result_path).sort_values(by="paramset").to_csv(
        session_result_path, index=False
    )


def __calculate_paramset_stats(paramset_dir: str, session_result_path: str):
    info = pd.read_csv(f"{paramset_dir}/paramset.csv")

    series_list = []
    for iter_csv in [
        f for f in os.listdir(paramset_dir) if re.match(r"iteration_\d+.csv", f)
    ]:
        df = pd.read_csv(f"{paramset_dir}/{iter_csv}")
        # The 1st column is the dissemination time
        series_list.append(pd.Series(df.iloc[:, 0]))

    series = pd.concat(series_list, ignore_index=True)
    stats = series.describe()
    result = {
        "paramset": info["paramset"].iloc[0],
        "num_nodes": info["num_nodes"].iloc[0],
        "peering_degree": info["peering_degree"].iloc[0],
        "min_queue_size": info["min_queue_size"].iloc[0],
        "transmission_rate": info["transmission_rate"].iloc[0],
        "num_sent_msgs": info["num_sent_msgs"].iloc[0],
        "num_senders": info["num_senders"].iloc[0],
        "queue_type": info["queue_type"].iloc[0],
        "num_iterations": info["num_iterations"].iloc[0],
        "dtime_count": stats["count"],
        "dtime_mean": stats["mean"],
        "dtime_std": stats["std"],
        "dtime_min": stats["min"],
        "dtime_25%": stats["25%"],
        "dtime_50%": stats["50%"],
        "dtime_75%": stats["75%"],
        "dtime_max": stats["max"],
    }
    assert result.keys() == set(RESULT_COLUMNS)
    pd.DataFrame([result]).to_csv(
        session_result_path, mode="a", header=False, index=False
    )
