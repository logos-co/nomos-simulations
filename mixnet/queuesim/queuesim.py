import concurrent.futures
import itertools
import os
import random
import time
from collections import defaultdict
from copy import deepcopy
from datetime import datetime, timedelta

import pandas as pd
import usim

from protocol.nomssip import NomssipConfig
from protocol.temporalmix import TemporalMixConfig, TemporalMixType
from queuesim.config import Config
from queuesim.paramset import (
    EXPERIMENT_TITLES,
    ExperimentID,
    ParameterSet,
    SessionID,
    build_parameter_sets,
)
from queuesim.simulation import Simulation
from sim.config import LatencyConfig, TopologyConfig

DEFAULT_CONFIG = Config(
    num_nodes=10,
    nomssip=NomssipConfig(
        peering_degree=3,
        transmission_rate_per_sec=10,
        msg_size=8,
        temporal_mix=TemporalMixConfig(
            mix_type=TemporalMixType.NONE,
            min_queue_size=10,
            seed_generator=random.Random(0),
        ),
        skip_sending_noise=True,
    ),
    topology=TopologyConfig(
        seed=random.Random(0),
    ),
    latency=LatencyConfig(
        min_latency_sec=0,
        max_latency_sec=0,
        seed=random.Random(0),
    ),
    num_sent_msgs=1,
    msg_interval_sec=0.1,
    num_senders=1,
    sender_generator=random.Random(0),
)


RESULT_COLUMNS = [
    "paramset",
    "num_nodes",
    "peering_degree",
    "min_queue_size",
    "transmission_rate",
    "num_sent_msgs",
    "num_senders",
    "queue_type",
    "num_iterations",
    "dtime_count",
    "dtime_mean",
    "dtime_std",
    "dtime_min",
    "dtime_25%",
    "dtime_50%",
    "dtime_75%",
    "dtime_max",
]


def run_session(
    exp_id: ExperimentID,
    session_id: SessionID,
    queue_type: TemporalMixType,
    outdir: str,
    from_paramset: int = 1,
):
    print("******************************************************************")
    print(f"{exp_id.name}: {session_id.name}: {EXPERIMENT_TITLES[exp_id]}")
    print(f"Queue type: {queue_type.name}")
    print("******************************************************************")

    # Create a directory and initialize a CSV file only with a header
    assert os.path.isdir(outdir)
    subdir = f"__WIP__queuesim_e{exp_id.value}s{session_id.value}_{queue_type.name}_{datetime.now().isoformat()}___DUR__"
    os.makedirs(f"{outdir}/{subdir}")
    session_result_path = f"{outdir}/{subdir}/session.csv"
    assert not os.path.exists(session_result_path)
    pd.DataFrame(columns=pd.Series(RESULT_COLUMNS)).to_csv(
        session_result_path, index=False
    )
    print(f"Initialized a CSV file: {session_result_path}")

    # Prepare all parameter sets of the session
    paramsets = build_parameter_sets(exp_id, session_id, queue_type)
    assert 1 <= from_paramset <= len(paramsets)

    # Run the simulations for each parameter set, using multi processes
    session_start_time = time.time()
    future_map: dict[
        concurrent.futures.Future[list[float]], tuple[int, ParameterSet, int]
    ] = dict()
    with concurrent.futures.ProcessPoolExecutor() as executor:
        # Submit all iterations of all parameter sets to the ProcessPoolExecutor
        for paramset_idx, paramset in enumerate(paramsets):
            paramset_id = paramset_idx + 1
            if paramset_id < from_paramset:
                continue
            future_map.update(__submit_iterations(paramset_id, paramset, executor))

        # Collect results of each iteration
        paramset_results: dict[int, tuple[set[int], list[float]]] = defaultdict(
            lambda: (set(), [])
        )
        paramsets_done: set[int] = set()
        for future in concurrent.futures.as_completed(future_map):
            paramset_id, paramset, iter_idx = future_map[future]
            paramset_results[paramset_id][0].add(iter_idx)
            paramset_results[paramset_id][1].extend(future.result())
            # If all iterations of the paramset are done, process the results
            if len(paramset_results[paramset_id][0]) == paramset.num_iterations:
                paramsets_done.add(paramset_id)
                print("================================================")
                print(f"ParamSet-{paramset_id} is done. Processing results...")
                print(
                    f"Total {len(paramsets_done)+(from_paramset-1)}/{len(paramsets)} paramsets have been done so far."
                )
                print("------------------------------------------------")
                __process_paramset_result(
                    paramset_id,
                    paramset,
                    paramset_results[paramset_id][1],
                    session_result_path,
                    f"{outdir}/{subdir}/paramset_{paramset_id}.csv",
                )
                print("================================================")

    session_elapsed_time = time.time() - session_start_time
    session_elapsed_time_str = __format_elapsed_time(session_elapsed_time)

    # Load the completed session CSV file, sort rows by paramset_id,
    # and overrite the file with the sorted rows.
    pd.read_csv(session_result_path).sort_values(by="paramset").to_csv(
        session_result_path, index=False
    )

    # Rename the WIP directory to the final name
    new_subdir = subdir.replace("__WIP__", "").replace(
        "__DUR__", session_elapsed_time_str
    )
    assert not os.path.exists(f"{outdir}/{new_subdir}")
    os.rename(f"{outdir}/{subdir}", f"{outdir}/{new_subdir}")

    print("******************************************************************")
    print(f"Session Elapsed Time: {session_elapsed_time_str}")
    print(f"Renamed the WIP directory to {outdir}/{new_subdir}")
    print("******************************************************************")


def __submit_iterations(
    paramset_id: int,
    paramset: ParameterSet,
    executor: concurrent.futures.ProcessPoolExecutor,
) -> dict[concurrent.futures.Future[list[float]], tuple[int, ParameterSet, int]]:
    """
    Submit all iterations of the given parameter set to the executor,
    so that they can be ran by the ProcessPoolExecutor.
    """
    # Prepare the configuration for the parameter set
    cfg = deepcopy(DEFAULT_CONFIG)
    paramset.apply_to(cfg)

    print(
        f"Scheduling {paramset.num_iterations} iterations for the paramset:{paramset_id}"
    )

    future_map: dict[
        concurrent.futures.Future[list[float]], tuple[int, ParameterSet, int]
    ] = dict()
    for i in range(paramset.num_iterations):
        # Update seeds for the current iteration
        # Deepcopy the cfg to avoid the same cfg instance between iteration jobs.
        iter_cfg = deepcopy(cfg)
        iter_cfg.nomssip.temporal_mix.seed_generator = random.Random(i)
        iter_cfg.topology.seed = random.Random(i)
        iter_cfg.latency.seed = random.Random(i)
        iter_cfg.sender_generator = random.Random(i)
        # Submit the iteration to the executor
        future = executor.submit(__run_iteration, iter_cfg)
        future_map[future] = (paramset_id, paramset, i)

    return future_map


def __run_iteration(cfg: Config) -> list[float]:
    """
    Run a single iteration of a certain parameter set.
    The iteration uses the independent uSim instance.
    """
    sim = Simulation(cfg)
    usim.run(sim.run())
    return sim.dissemination_times


def __process_paramset_result(
    paramset_id: int,
    paramset: ParameterSet,
    dissemination_times: list[float],
    session_result_path: str,
    paramset_result_path: str,
):
    """
    Convert the result into a pd.Series, store the Series into a CSV file,
    and append the summary of Series to the session CSV file.
    """
    series = pd.Series(dissemination_times)
    stats = series.describe()
    result = {
        "paramset": paramset_id,
        "num_nodes": paramset.num_nodes,
        "peering_degree": paramset.peering_degree,
        "min_queue_size": paramset.min_queue_size,
        "transmission_rate": paramset.transmission_rate,
        "num_sent_msgs": paramset.num_sent_msgs,
        "num_senders": paramset.num_senders,
        "queue_type": paramset.queue_type.name,
        "num_iterations": paramset.num_iterations,
        "dtime_count": stats["count"],
        "dtime_mean": stats["mean"],
        "dtime_std": stats["std"],
        "dtime_min": stats["min"],
        "dtime_25%": stats["25%"],
        "dtime_50%": stats["50%"],
        "dtime_75%": stats["75%"],
        "dtime_max": stats["max"],
    }
    assert result.keys() == set(RESULT_COLUMNS)
    pd.DataFrame([result]).to_csv(
        session_result_path, mode="a", header=False, index=False
    )
    print(f"Appended a row to {session_result_path}")
    series.to_csv(paramset_result_path, header=False, index=False)
    print(f"Stored the dissemination times to {paramset_result_path}")


def __format_elapsed_time(elapsed_time: float) -> str:
    td = timedelta(seconds=elapsed_time)
    hours, reminder = divmod(td.seconds, 3600)
    minutes, seconds = divmod(reminder, 60)
    return f"{td.days}d{hours:02}h{minutes:02}m{seconds:02}s"
