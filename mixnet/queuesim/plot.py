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


def __draw_plot_by_queue_type(
    df: pd.DataFrame, title: str, out_path: str, legend_columns: list[str] = PARAM_SET
):
    # Add a column that will be used as a legend
    def __create_legend_value(row):
        legend = ""
        for i, param in enumerate(legend_columns):
            if i > 0:
                legend += ", "
            legend += f"{param}:{row[param]}"
        return legend

    df["legend"] = df.apply(__create_legend_value, axis=1)

    # Prepare DataFrame in long format for seaborn boxplot
    long_format_df = pd.melt(
        df,
        id_vars=["queue_type", "legend"],
        value_vars=BOXPLOT_VALUE_VARS,
        var_name="dtime_metric",
        value_name="dtime",
    )

    # Plotting
    plt.figure(figsize=(15, 10))
    sns.boxplot(data=long_format_df, x="queue_type", y="dtime", hue="legend")
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
    print(f"Saved plot to {out_path}")


def draw_cross_experiment_plots(dfs: list[pd.DataFrame], outdir: str):
    assert os.path.exists(outdir)
    assert len(dfs) == len(ExperimentID)

    # Common filtering conditions
    common_conditions = {
        "num_nodes": 80,
        "peering_degree": 16,
        "min_queue_size": 40,
        "transmission_rate": 40,
    }
    # Define the filtering conditions for each DataFrame
    conditions = [
        {"num_senders": 1, "num_sent_msgs": 1},
        {"num_senders": 1, "num_sent_msgs": 8},
        {"num_senders": 8, "num_sent_msgs": 1},
        {"num_senders": 8, "num_sent_msgs": 8},
    ]

    filtered_dfs: list[pd.DataFrame] = []
    for exp_idx, (df, condition) in enumerate(zip(dfs, conditions)):
        # Combine common and specific conditions
        all_conditions = {**common_conditions, **condition}
        # Filter the DataFrame
        filtered_df = pd.DataFrame(
            df[
                (df["num_nodes"] == all_conditions["num_nodes"])
                & (df["peering_degree"] == all_conditions["peering_degree"])
                & (df["min_queue_size"] == all_conditions["min_queue_size"])
                & (df["transmission_rate"] == all_conditions["transmission_rate"])
                & (df["num_senders"] == all_conditions["num_senders"])
                & (df["num_sent_msgs"] == all_conditions["num_sent_msgs"])
            ]
        )
        filtered_df["experiment"] = ExperimentID(exp_idx + 1).value
        filtered_dfs.append(
            pd.DataFrame(
                filtered_df[
                    ["experiment"] + PARAM_SET + ["queue_type"] + BOXPLOT_VALUE_VARS
                ]
            )
        )

    __draw_plot_by_queue_type(
        pd.concat(filtered_dfs),
        "Comparisons by Experiment and Queue Type",
        f"{outdir}/cross_experiment_plot.png",
        legend_columns=["experiment"] + PARAM_SET,
    )
