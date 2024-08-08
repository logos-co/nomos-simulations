from __future__ import annotations

import concurrent.futures
import tempfile

from protocol.temporalmix import TemporalMixType
from queuesim.paramset import ParameterSet
from queuesim.queuesim import IterationInfo, _submit_iterations


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
        future_map: dict[concurrent.futures.Future[bool], IterationInfo] = dict()
        with concurrent.futures.ProcessPoolExecutor(
            max_workers=num_workers
        ) as executor:
            future_map.update(
                _submit_iterations(
                    paramset_id=1, paramset=paramset, executor=executor, outdir=tmpdir
                )
            )

        # Wait until all iterations are done
        results: list[tuple[IterationInfo, bool]] = []
        for future in concurrent.futures.as_completed(future_map):
            iter = future_map[future]
            succeeded = future.result()
            if not succeeded:
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                print("ITERATION FAILED: See the err file")
                print(iter)
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
            else:
                print("------------------------------------------------")
                print("ITERATION SUCCEEDED")
                print(iter)
                print("------------------------------------------------")

            results.append((iter, succeeded))
            # If all iterations of the paramset are done, print a log
            if len(results) == iter.paramset.num_iterations:
                print("ALL ITERATIONS DONE")
