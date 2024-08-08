import argparse
import glob
import os

from protocol.temporalmix import TemporalMixType

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Merge multiple `all_results.csv` files into a single CSV file.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--exp-id", type=int, required=True, help="Experiment ID (>=1)")
    parser.add_argument(
        "--session-id", type=float, required=True, help="Session ID (>=1)"
    )
    parser.add_argument("--indir", type=str, required=True, help="input directory")
    parser.add_argument(
        "--out-csv-path", type=str, required=True, help="output CSV file path"
    )
    args = parser.parse_args()

    pattern = os.path.join(args.indir, f"queuesim_e{args.exp_id}s{args.session_id}_*")
    files = glob.glob(pattern)
    assert len(TemporalMixType) == len(files), f"{len(TemporalMixType)} != {len(files)}"

    paths = [""] * len(TemporalMixType)
    for file in files:
        for i, queue_type in enumerate(TemporalMixType):
            if f"_{queue_type.name}_" in file:
                assert paths[i] == ""
                paths[i] = file
                break
    assert all(path != "" for path in paths)

    with open(args.out_csv_path, "w", newline="") as output:
        for i, path in enumerate(paths):
            with open(f"{path}/session.csv") as input:
                header = input.readline()
                if i == 0:
                    output.write(header)

                for line in input:
                    output.write(line)

    print(f"Saved to {args.out_csv_path}")
