import argparse

import pandas as pd

from queuesim.paramset import ExperimentID, SessionID
from queuesim.plot import draw_plots

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Draw plots from a merged CSV file.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--exp-id", type=int, required=True, help="Experiment ID (>=1)")
    parser.add_argument(
        "--session-id", type=float, required=True, help="Session ID (>=1)"
    )
    parser.add_argument(
        "--csv-path", type=str, required=True, help="input CSV file path"
    )
    parser.add_argument("--outdir", type=str, required=True, help="output directory")
    args = parser.parse_args()

    exp_id: ExperimentID = ExperimentID(args.exp_id)
    session_id: SessionID = SessionID(args.session_id)
    df = pd.read_csv(args.csv_path)

    draw_plots(df, exp_id, session_id, args.outdir)
