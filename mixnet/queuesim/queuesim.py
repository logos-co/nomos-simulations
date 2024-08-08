from __future__ import annotations

import concurrent.futures
import os
import random
import time
import traceback
from copy import deepcopy
from dataclasses import dataclass
from datetime import datetime
from typing import Counter

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
from queuesim.util import format_elapsed_time
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

PARAMSET_INFO_COLUMNS = [
    "paramset",
    "num_nodes",
    "peering_degree",
    "min_queue_size",
    "transmission_rate",
    "num_sent_msgs",
    "num_senders",
    "queue_type",
    "num_iterations",
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

    # Prepare all parameter sets of the session
    paramsets = build_parameter_sets(exp_id, session_id, queue_type)
    assert 1 <= from_paramset <= len(paramsets)

    # Run the simulations for each parameter set, using multi processes
    session_start_time = time.time()
    future_map: dict[concurrent.futures.Future[tuple[bool, float]], IterationInfo] = (
        dict()
    )
    total_cores = os.cpu_count()
    assert total_cores is not None
    max_workers = max(1, total_cores - 1)
    with concurrent.futures.ProcessPoolExecutor(max_workers=max_workers) as executor:
        # Submit all iterations of all parameter sets to the ProcessPoolExecutor
        for paramset_idx, paramset in enumerate(paramsets):
            paramset_id = paramset_idx + 1
            if paramset_id < from_paramset:
                continue
            paramset_dir = f"{outdir}/{subdir}/paramset_{paramset_id}"
            os.makedirs(paramset_dir)
            __save_paramset_info(paramset_id, paramset, f"{paramset_dir}/paramset.csv")
            future_map.update(
                _submit_iterations(paramset_id, paramset, executor, paramset_dir)
            )

        # Wait until all parameter sets are done
        iterations_done: Counter[int] = Counter()  # per paramset_id
        paramsets_done: set[int] = set()
        for future in concurrent.futures.as_completed(future_map):
            iter = future_map[future]
            succeeded, _ = future.result()
            if not succeeded:
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                print("ITERATION FAILED: See the err file")
                print(iter)
                print("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")

            iterations_done.update([iter.paramset_id])
            # If all iterations of the paramset are done, print a log
            if iterations_done[iter.paramset_id] == iter.paramset.num_iterations:
                paramsets_done.add(iter.paramset_id)
                print("================================================")
                print(
                    f"ParamSet-{iter.paramset_id} is done. Total {len(paramsets_done)+(from_paramset-1)}/{len(paramsets)} paramsets have been done so far."
                )
                print("================================================")

    session_elapsed_time = time.time() - session_start_time
    session_elapsed_time_str = format_elapsed_time(session_elapsed_time)

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


def __save_paramset_info(paramset_id: int, paramset: ParameterSet, path: str):
    assert not os.path.exists(path)
    info = {
        "paramset": paramset_id,
        "num_nodes": paramset.num_nodes,
        "peering_degree": paramset.peering_degree,
        "min_queue_size": paramset.min_queue_size,
        "transmission_rate": paramset.transmission_rate,
        "num_sent_msgs": paramset.num_sent_msgs,
        "num_senders": paramset.num_senders,
        "queue_type": paramset.queue_type.name,
        "num_iterations": paramset.num_iterations,
    }
    assert info.keys() == set(PARAMSET_INFO_COLUMNS)
    pd.DataFrame([info]).to_csv(path, mode="w", header=True, index=False)


def _submit_iterations(
    paramset_id: int,
    paramset: ParameterSet,
    executor: concurrent.futures.ProcessPoolExecutor,
    outdir: str,
) -> dict[concurrent.futures.Future[tuple[bool, float]], IterationInfo]:
    """
    Submit all iterations of the given parameter set to the executor,
    so that they can be ran by the ProcessPoolExecutor.
    """
    assert os.path.exists(outdir)

    # Prepare the configuration for the parameter set
    cfg = deepcopy(DEFAULT_CONFIG)
    paramset.apply_to(cfg)

    print(
        f"Scheduling {paramset.num_iterations} iterations for the paramset:{paramset_id}"
    )

    future_map: dict[concurrent.futures.Future[tuple[bool, float]], IterationInfo] = (
        dict()
    )
    for i in range(paramset.num_iterations):
        # Update seeds for the current iteration
        # Deepcopy the cfg to avoid the same cfg instance between iteration jobs.
        iter_cfg = deepcopy(cfg)
        iter_cfg.nomssip.temporal_mix.seed_generator = random.Random(i)
        iter_cfg.topology.seed = random.Random(i)
        iter_cfg.latency.seed = random.Random(i)
        iter_cfg.sender_generator = random.Random(i)
        # Submit the iteration to the executor
        out_csv_path = f"{outdir}/iteration_{i}.csv"
        err_path = f"{outdir}/iteration_{i}.err"
        future = executor.submit(_run_iteration, iter_cfg, out_csv_path, err_path)
        future_map[future] = IterationInfo(
            paramset_id, paramset, i, out_csv_path, err_path
        )

    return future_map


def _run_iteration(cfg: Config, out_csv_path: str, err_path: str) -> tuple[bool, float]:
    """
    Run a single iteration of a certain parameter set.
    The iteration uses the independent uSim instance.
    Returns False if exception happened.
    """
    start_time = time.time()
    try:
        sim = Simulation(cfg)
        usim.run(sim.run(out_csv_path))
        return True, time.time() - start_time
    except BaseException as e:
        with open(err_path, "w") as f:
            traceback.print_exc(file=f)
        return False, time.time() - start_time


@dataclass
class IterationInfo:
    paramset_id: int
    paramset: ParameterSet
    iteration_idx: int
    out_csv_path: str
    err_path: str
