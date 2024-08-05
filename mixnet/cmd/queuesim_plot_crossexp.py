import argparse

import pandas as pd

from queuesim.paramset import ExperimentID, SessionID
from queuesim.plot import draw_cross_experiment_plots, draw_plots

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Draw cross-experiment plots",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "--csv-paths", type=str, required=True, help="Experiment result CSV paths"
    )
    parser.add_argument("--outdir", type=str, required=True, help="output directory")
    args = parser.parse_args()

    dfs = [pd.read_csv(path) for path in args.csv_paths.split(",")]
    draw_cross_experiment_plots(dfs, args.outdir)
