import random
from copy import deepcopy
from unittest import TestCase

from protocol.nomssip import NomssipConfig
from protocol.temporalmix import TemporalMixConfig, TemporalMixType
from queuesim.config import Config
from queuesim.paramset import (
    ExperimentID,
    ParameterSet,
    SessionID,
    _is_min_queue_size_applicable,
    build_parameter_sets,
)
from sim.config import LatencyConfig, TopologyConfig


class TestParameterSet(TestCase):
    def test_apply_to_config(self):
        paramset = ParameterSet(
            id=1,
            num_nodes=10000,
            peering_degree=20000,
            min_queue_size=30000,
            transmission_rate=40000,
            num_sent_msgs=50000,
            num_senders=60000,
            queue_type=TemporalMixType.NOISY_COIN_FLIPPING,
            num_iterations=70000,
        )
        config = deepcopy(SAMPLE_CONFIG)
        paramset.apply_to(config)
        self.assertEqual(paramset.num_nodes, config.num_nodes)
        self.assertEqual(paramset.peering_degree, config.nomssip.peering_degree)
        self.assertEqual(
            paramset.min_queue_size, config.nomssip.temporal_mix.min_queue_size
        )
        self.assertEqual(
            paramset.transmission_rate, config.nomssip.transmission_rate_per_sec
        )
        self.assertEqual(paramset.num_sent_msgs, config.num_sent_msgs)
        self.assertEqual(paramset.num_senders, config.num_senders)
        self.assertEqual(paramset.queue_type, config.nomssip.temporal_mix.mix_type)

    def test_build_parameter_sets(self):
        cases = {
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_1): pow(3, 4),
            (ExperimentID.EXPERIMENT_2, SessionID.SESSION_1): pow(3, 5),
            (ExperimentID.EXPERIMENT_3, SessionID.SESSION_1): pow(3, 5),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_1): pow(3, 6),
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_2): pow(3, 4),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_2): pow(3, 6),
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_2_1): pow(3, 3),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_2_1): pow(3, 4),
        }

        for queue_type in TemporalMixType:
            for (exp_id, session_id), expected_cnt in cases.items():
                sets = build_parameter_sets(exp_id, session_id, queue_type)

                # Check if the number of parameter sets is correct
                if not _is_min_queue_size_applicable(queue_type):
                    expected_cnt //= 3
                self.assertEqual(expected_cnt, len(sets), f"{exp_id}: {session_id}")

                # Check if all parameter sets are unique
                self.assertEqual(
                    len(sets),
                    len(set(list(map(str, sets)))),
                    f"{exp_id}: {session_id}",
                )

                # Check if paramset IDs are correct.
                # ID starts from 1.
                if _is_min_queue_size_applicable(queue_type):
                    for i, paramset in enumerate(sets):
                        self.assertEqual(i + 1, paramset.id, f"{exp_id}: {session_id}")

    def test_parameter_set_id_consistency(self):
        cases = [
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_1),
            (ExperimentID.EXPERIMENT_2, SessionID.SESSION_1),
            (ExperimentID.EXPERIMENT_3, SessionID.SESSION_1),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_1),
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_2),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_2),
            (ExperimentID.EXPERIMENT_1, SessionID.SESSION_2_1),
            (ExperimentID.EXPERIMENT_4, SessionID.SESSION_2_1),
        ]

        for exp_id, session_id in cases:
            sets_with_min_queue_size = build_parameter_sets(
                exp_id, session_id, TemporalMixType.PURE_COIN_FLIPPING
            )
            sets_without_min_queue_size = build_parameter_sets(
                exp_id, session_id, TemporalMixType.NONE
            )

            for i, paramset in enumerate(sets_with_min_queue_size):
                self.assertEqual(i + 1, paramset.id, f"{exp_id}: {session_id}")

            for set in sets_without_min_queue_size:
                # To compare ParameterSet instances, use the same queue type.
                modified_set = deepcopy(set)
                modified_set.queue_type = TemporalMixType.PURE_COIN_FLIPPING
                self.assertEqual(
                    sets_with_min_queue_size[set.id - 1],
                    modified_set,
                    f"{exp_id}: {session_id}",
                )


SAMPLE_CONFIG = Config(
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