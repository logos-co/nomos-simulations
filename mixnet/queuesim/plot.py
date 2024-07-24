import os

import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns

from queuesim.paramset import ExperimentID, SessionID

BOXPLOT_VALUE_VARS = [
    "dtime_min",
    "dtime_25%",
    "dtime_50%",
    "dtime_mean",
    "dtime_75%",
    "dtime_max",
]

PARAM_SET = [
    "num_nodes",
    "peering_degree",
    "min_queue_size",
    "transmission_rate",
    "num_sent_msgs",
    "num_senders",
]


def draw_plots(
    df: pd.DataFrame, exp_id: ExperimentID, session_id: SessionID, outdir: str
):
    assert os.path.exists(outdir)
    __overview_by_queue_type(df, exp_id, session_id, f"{outdir}/plot_overview.png")
    num_nodes = int(df["num_nodes"].min())
    for param in ["peering_degree", "min_queue_size", "transmission_rate"]:
        __impact_of_param_by_queue_type(
            df, exp_id, session_id, num_nodes, param, f"{outdir}/plot_{param}.png"
        )

    if exp_id == ExperimentID(2) or exp_id == ExperimentID(4):
        __impact_of_param_by_queue_type(
            df,
            exp_id,
            session_id,
            num_nodes,
            "num_sent_msgs",
            f"{outdir}/plot_num_sent_msgs.png",
        )

    if exp_id == ExperimentID(3) or exp_id == ExperimentID(4):
        __impact_of_param_by_queue_type(
            df,
            exp_id,
            session_id,
            num_nodes,
            "num_senders",
            f"{outdir}/plot_num_senders.png",
        )


def __param_set_legend(row):
    legend = ""
    for i, param in enumerate(PARAM_SET):
        if i > 0:
            legend += ", "
        legend += f"{param}:{row[param]}"
    return legend


def __overview_by_queue_type(
    df: pd.DataFrame,
    exp_id: ExperimentID,
    session_id: SessionID,
    out_path: str,
):
    df = df.drop_duplicates(subset=["num_nodes", "queue_type"])
    print(df)
    __draw_plot_by_queue_type(
        df, f"{exp_id.name}: {session_id.name}: Overview", out_path
    )


def __impact_of_param_by_queue_type(
    df: pd.DataFrame,
    exp_id: ExperimentID,
    session_id: SessionID,
    num_nodes: int,
    param: str,
    out_path: str,
):
    df = pd.DataFrame(df[df["num_nodes"] == num_nodes])
    df = df.drop_duplicates(subset=[param, "queue_type"])
    print(df)
    __draw_plot_by_queue_type(
        df,
        f"{exp_id.name}: {session_id.name}: Impact of {param} ({num_nodes} nodes)",
        out_path,
    )


def __draw_plot_by_queue_type(df: pd.DataFrame, title: str, out_path: str):
    # Add a column that will be used as a legend
    df["parameter_set"] = df.apply(__param_set_legend, axis=1)

    # Prepare DataFrame in long format for seaborn boxplot
    long_format_df = pd.melt(
        df,
        id_vars=["queue_type", "parameter_set"],
        value_vars=BOXPLOT_VALUE_VARS,
        var_name="dtime_metric",
        value_name="dtime",
    )

    # Plotting
    plt.figure(figsize=(15, 10))
    sns.boxplot(data=long_format_df, x="queue_type", y="dtime", hue="parameter_set")
    plt.title(title)
    plt.xlabel("Queue Type")
    plt.ylabel("Dissemination Time")
    plt.legend(loc="upper right", ncol=1)

    # Adding vertical grid lines between x elements
    plt.grid(axis="y")
    plt.gca().set_xticks(
        [i - 0.5 for i in range(1, len(df["queue_type"].unique()))], minor=True
    )
    plt.grid(which="minor", axis="x", linestyle="--")

    plt.tight_layout()

    # Save the plot as a PNG file
    assert not os.path.exists(out_path)
    plt.savefig(out_path)
    plt.draw()
