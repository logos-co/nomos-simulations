import argparse

from queuesim.statistics import calculate_session_stats

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Calculate statistics for a session.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--dir", type=str, required=True, help="session directory")
    args = parser.parse_args()

    calculate_session_stats(args.dir)
