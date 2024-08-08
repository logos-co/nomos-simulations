from __future__ import annotations

import concurrent.futures
import tempfile
import time

import pandas as pd

from protocol.temporalmix import TemporalMixType
from queuesim.paramset import ParameterSet
from queuesim.queuesim import IterationInfo, _submit_iterations
from queuesim.util import format_elapsed_time


def benchmark(num_workers: int):
    paramset = ParameterSet(
        num_nodes=100,
        peering_degree=4,
        min_queue_size=10,
        transmission_rate=10,
        num_sent_msgs=100,
        num_senders=10,
        queue_type=TemporalMixType.NONE,
        num_iterations=100,
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        start_time = time.time()

        future_map: dict[
            concurrent.futures.Future[tuple[bool, float]], IterationInfo
        ] = dict()
        with concurrent.futures.ProcessPoolExecutor(
            max_workers=num_workers
        ) as executor:
            future_map.update(
                _submit_iterations(
                    paramset_id=1, paramset=paramset, executor=executor, outdir=tmpdir
                )
            )

        # Wait until all iterations are done
        iter_durations: list[float] = []
        for future in concurrent.futures.as_completed(future_map):
            iter = future_map[future]
            succeeded, duration = future.result()
            if not succeeded:
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                print("ITERATION FAILED: See the err file")
                print(iter)
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")

            iter_durations.append(duration)
            # If all iterations of the paramset are done, print a log
            if len(iter_durations) == iter.paramset.num_iterations:
                iter_durations_series = pd.Series(iter_durations)
                print("================================================")
                print("ALL ITERATIONS DONE")
                print(f"NUM_WORKERS: {num_workers}")
                print(f"PARAMSET: {paramset}")
                print(
                    f"TOTAL DURATION: {format_elapsed_time(time.time() - start_time)}"
                )
                print("ITERATION DURATIONS:")
                print(iter_durations_series.describe())
                print("================================================")

                break
