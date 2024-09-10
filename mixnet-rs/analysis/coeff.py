import argparse

import matplotlib.pyplot as plt
import pandas as pd

from common import MARKERS, X_FIELDS


def analyze(path: str, outdir: str):
    data = pd.read_csv(path)
    for x_field in X_FIELDS:
        analyze_versus(data, x_field, outdir)


def analyze_versus(data: pd.DataFrame, x_field: str, outdir: str):
    # Group by both x_field and queue_type, then select the row with the largest paramset for each group
    max_paramset_data = data.loc[
        data.groupby([x_field, "queue_type"])["paramset"].idxmax()
    ]

    all_fields = [
        ["strong_coeff_min", "casual_coeff_min", "weak_coeff_min"],
        ["strong_coeff_mean", "casual_coeff_mean", "weak_coeff_mean"],
        ["strong_coeff_max", "casual_coeff_max", "weak_coeff_max"],
    ]

    # Display the plots
    fig, ax = plt.subplots(3, 3, figsize=(20, 10))
    for ax_col, fields in enumerate(all_fields):
        max_y = 0
        for field in fields:
            max_y = max(max_y, max_paramset_data[field].max())
        max_y = min(max_y, 5000)  # hard limit

        for ax_row, field in enumerate(fields):
            for queue_type in max_paramset_data["queue_type"].unique():
                queue_data = max_paramset_data[
                    max_paramset_data["queue_type"] == queue_type
                ]
                x_values = queue_data[x_field]
                y_values = queue_data[field]
                ax[ax_row][ax_col].plot(
                    x_values,
                    y_values,
                    label=queue_type,
                    marker=MARKERS[queue_type],
                )

            ax[ax_row][ax_col].set_title(f"{field} vs {x_field}", fontsize=10)
            ax[ax_row][ax_col].set_xlabel(x_field)
            ax[ax_row][ax_col].set_ylabel(field)
            if ax_row == 0 and ax_col == len(all_fields) - 1:
                ax[ax_row][ax_col].legend(bbox_to_anchor=(1, 1), loc="upper left")
            ax[ax_row][ax_col].grid(True)
            ax[ax_row][ax_col].set_ylim(-10, max_y * 1.05)

    plt.tight_layout()
    fig.savefig(f"{outdir}/coeff_vs_{x_field}.png")

    # Display the table of values
    # Create a table with x_field, queue_type, and coefficients
    flatten_fields = [
        field for zipped_fields in zip(*all_fields) for field in zipped_fields
    ]
    columns = [x_field, "queue_type"] + flatten_fields
    table_data = max_paramset_data[columns].sort_values(by=[x_field, "queue_type"])
    # Prepare to display values with only 2 decimal places
    table_data[fields] = table_data[fields].map(
        lambda x: f"{x:.2e}" if abs(x) >= 1e6 else f"{x:.2f}"
    )
    # Display the table as a separate subplot
    fig_table, ax_table = plt.subplots(
        figsize=(len(columns) * 1.8, len(table_data) * 0.3)
    )
    ax_table.axis("off")  # Turn off the axis
    table = ax_table.table(
        cellText=table_data.values,
        colLabels=table_data.columns,
        cellLoc="center",
        loc="center",
    )
    table.auto_set_font_size(False)
    table.set_fontsize(10)
    table.scale(1, 1.5)
    for i in range(len(table_data.columns)):
        table.auto_set_column_width(i)

    fig_table.savefig(f"{outdir}/coeff_vs_{x_field}_table.png")

    plt.draw()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Aggregate the results of all paramsets of an experiment"
    )
    parser.add_argument("path", type=str, help="dir path")
    parser.add_argument("outdir", type=str, help="output dir path")
    args = parser.parse_args()
    analyze(args.path, args.outdir)
