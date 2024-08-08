import argparse

from protocol.temporalmix import TemporalMixType
from queuesim.paramset import ExperimentID, SessionID
from queuesim.queuesim import run_session

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Measure the message dissemination time with various configurations.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--exp-id", type=int, required=True, help="Experiment ID (>=1)")
    parser.add_argument(
        "--session-id", type=int, required=True, help="Session ID (>=1)"
    )
    parser.add_argument(
        "--queue-type",
        type=str,
        required=True,
        help=f"Queue type: {' | '.join([t.value for t in TemporalMixType])}",
    )
    parser.add_argument("--num-workers", type=int, required=True, help="num workers")
    parser.add_argument("--outdir", type=str, required=True, help="output directory")
    parser.add_argument(
        "--from-paramset",
        type=int,
        required=False,
        default=1,
        help="A parameter set ID (>=1) to start from",
    )
    args = parser.parse_args()

    run_session(
        ExperimentID(args.exp_id),
        SessionID(args.session_id),
        TemporalMixType(args.queue_type),
        args.num_workers,
        args.outdir,
        args.from_paramset,
    )
    print("All simulations completed!")
