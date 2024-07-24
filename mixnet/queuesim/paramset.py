from __future__ import annotations

import itertools
from dataclasses import dataclass
from enum import Enum

from protocol.temporalmix import TemporalMixType
from queuesim.config import Config


class ExperimentID(Enum):
    EXPERIMENT_1 = 1
    EXPERIMENT_2 = 2
    EXPERIMENT_3 = 3
    EXPERIMENT_4 = 4


class SessionID(Enum):
    SESSION_1 = 1
    SESSION_2 = 2


EXPERIMENT_TITLES: dict[ExperimentID, str] = {
    ExperimentID.EXPERIMENT_1: "Single Sender - Single Message",
    ExperimentID.EXPERIMENT_2: "Single Sender - Multiple Messages",
    ExperimentID.EXPERIMENT_3: "Multiple Senders - Single Message",
    ExperimentID.EXPERIMENT_4: "Multiple Senders - Multiple Messages",
}


@dataclass
class ParameterSet:
    num_nodes: int
    peering_degree: int
    min_queue_size: int
    transmission_rate: int
    num_sent_msgs: int
    num_senders: int
    queue_type: TemporalMixType
    num_iterations: int

    def apply_to(self, cfg: Config) -> None:
        cfg.num_nodes = self.num_nodes
        cfg.nomssip.peering_degree = self.peering_degree
        cfg.nomssip.temporal_mix.min_queue_size = self.min_queue_size
        cfg.nomssip.transmission_rate_per_sec = self.transmission_rate
        cfg.num_sent_msgs = self.num_sent_msgs
        cfg.num_senders = self.num_senders
        cfg.nomssip.temporal_mix.mix_type = self.queue_type


def build_parameter_sets(
    exp_id: ExperimentID, session_id: SessionID, queue_type: TemporalMixType
) -> list[ParameterSet]:
    match session_id:
        case SessionID.SESSION_1:
            return __build_session_1_parameter_sets(exp_id, queue_type)
        case SessionID.SESSION_2:
            return __build_session_2_parameter_sets(exp_id, queue_type)
        case _:
            raise ValueError(f"Unknown session ID: {session_id}")


def __build_session_1_parameter_sets(
    exp_id: ExperimentID, queue_type: TemporalMixType
) -> list[ParameterSet]:
    sets: list[ParameterSet] = []

    for num_nodes in [20, 40, 80]:
        peering_degree_list = [num_nodes // 5, num_nodes // 4, num_nodes // 2]
        min_queue_size_list = [num_nodes // 2, num_nodes, num_nodes * 2]
        transmission_rate_list = [num_nodes // 2, num_nodes, num_nodes * 2]
        num_sent_msgs_list = [8, 16, 32]
        num_senders_list = [num_nodes // 10, num_nodes // 5, num_nodes // 2]
        num_iterations = num_nodes // 2

        match exp_id:
            case ExperimentID.EXPERIMENT_1:
                for (
                    peering_degree,
                    min_queue_size,
                    transmission_rate,
                ) in itertools.product(
                    peering_degree_list,
                    min_queue_size_list,
                    transmission_rate_list,
                ):
                    sets.append(
                        ParameterSet(
                            num_nodes=num_nodes,
                            peering_degree=peering_degree,
                            min_queue_size=min_queue_size,
                            transmission_rate=transmission_rate,
                            num_sent_msgs=1,
                            num_senders=1,
                            queue_type=queue_type,
                            num_iterations=num_iterations,
                        )
                    )
            case ExperimentID.EXPERIMENT_2:
                for (
                    peering_degree,
                    min_queue_size,
                    transmission_rate,
                    num_sent_msgs,
                ) in itertools.product(
                    peering_degree_list,
                    min_queue_size_list,
                    transmission_rate_list,
                    num_sent_msgs_list,
                ):
                    sets.append(
                        ParameterSet(
                            num_nodes=num_nodes,
                            peering_degree=peering_degree,
                            min_queue_size=min_queue_size,
                            transmission_rate=transmission_rate,
                            num_sent_msgs=num_sent_msgs,
                            num_senders=1,
                            queue_type=queue_type,
                            num_iterations=num_iterations,
                        )
                    )
            case ExperimentID.EXPERIMENT_3:
                for (
                    peering_degree,
                    min_queue_size,
                    transmission_rate,
                    num_senders,
                ) in itertools.product(
                    peering_degree_list,
                    min_queue_size_list,
                    transmission_rate_list,
                    num_senders_list,
                ):
                    sets.append(
                        ParameterSet(
                            num_nodes=num_nodes,
                            peering_degree=peering_degree,
                            min_queue_size=min_queue_size,
                            transmission_rate=transmission_rate,
                            num_sent_msgs=1,
                            num_senders=num_senders,
                            queue_type=queue_type,
                            num_iterations=num_iterations,
                        )
                    )
            case ExperimentID.EXPERIMENT_4:
                for (
                    peering_degree,
                    min_queue_size,
                    transmission_rate,
                    num_sent_msgs,
                    num_senders,
                ) in itertools.product(
                    peering_degree_list,
                    min_queue_size_list,
                    transmission_rate_list,
                    num_sent_msgs_list,
                    num_senders_list,
                ):
                    sets.append(
                        ParameterSet(
                            num_nodes=num_nodes,
                            peering_degree=peering_degree,
                            min_queue_size=min_queue_size,
                            transmission_rate=transmission_rate,
                            num_sent_msgs=num_sent_msgs,
                            num_senders=num_senders,
                            queue_type=queue_type,
                            num_iterations=num_iterations,
                        )
                    )
            case _:
                raise ValueError(f"Unknown experiment ID: {exp_id}")

    return sets


def __build_session_2_parameter_sets(
    exp_id: ExperimentID, queue_type: TemporalMixType
) -> list[ParameterSet]:
    sets: list[ParameterSet] = []

    for num_nodes in [100, 1000, 10000]:
        peering_degree_list = [4, 8, 16]
        min_queue_size_list = [10, 50, 100]
        transmission_rate_list = [1, 10, 100]
        num_iterations = 20

        match exp_id:
            case ExperimentID.EXPERIMENT_1:
                for (
                    peering_degree,
                    min_queue_size,
                    transmission_rate,
                ) in itertools.product(
                    peering_degree_list,
                    min_queue_size_list,
                    transmission_rate_list,
                ):
                    sets.append(
                        ParameterSet(
                            num_nodes=num_nodes,
                            peering_degree=peering_degree,
                            min_queue_size=min_queue_size,
                            transmission_rate=transmission_rate,
                            num_sent_msgs=1,
                            num_senders=1,
                            queue_type=queue_type,
                            num_iterations=num_iterations,
                        )
                    )
            case _:
                raise NotImplementedError(
                    f"Experiment {exp_id} not implemented for session 2"
                )

    return sets
