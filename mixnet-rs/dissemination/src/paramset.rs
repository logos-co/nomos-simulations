use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

use crate::queue::QueueType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExperimentId {
    Experiment1 = 1,
    Experiment2 = 2,
    Experiment3 = 3,
    Experiment4 = 4,
    Experiment5 = 5,
}

impl std::str::FromStr for ExperimentId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Experiment1" => Ok(ExperimentId::Experiment1),
            "2" | "Experiment2" => Ok(ExperimentId::Experiment2),
            "3" | "Experiment3" => Ok(ExperimentId::Experiment3),
            "4" | "Experiment4" => Ok(ExperimentId::Experiment4),
            "5" | "Experiment5" => Ok(ExperimentId::Experiment5),
            _ => Err(format!("Invalid experiment ID: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SessionId {
    Session1 = 1,
    Session2 = 2,
    Session2_1 = 21,
    Session3 = 3,
}

impl std::str::FromStr for SessionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Session1" => Ok(SessionId::Session1),
            "2" | "Session2" => Ok(SessionId::Session2),
            "2.1" | "Session21" => Ok(SessionId::Session2_1),
            "3" | "Session3" => Ok(SessionId::Session3),
            _ => Err(format!("Invalid session ID: {}", s)),
        }
    }
}

pub const PARAMSET_CSV_COLUMNS: &[&str] = &[
    "paramset",
    "num_nodes",
    "peering_degree",
    "min_queue_size",
    "transmission_rate",
    "num_sent_msgs",
    "num_senders",
    "random_senders_every_time",
    "queue_type",
    "num_iterations",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParamSet {
    pub id: u16,
    pub num_nodes: u32,
    pub peering_degree_rates: PeeringDegreeRates,
    pub min_queue_size: u16,
    pub transmission_rate: u16,
    pub num_sent_msgs: u32,
    pub num_senders: u32,
    pub random_senders_every_time: bool,
    pub queue_type: QueueType,
    pub num_iterations: usize,
}

// peering_degree -> rate
// Use Vec instead of HashMap to avoid unexpected undeterministic behavior
type PeeringDegreeRates = Vec<(u32, f32)>;

impl ParamSet {
    pub fn new_all_paramsets(
        exp_id: ExperimentId,
        session_id: SessionId,
        queue_type: QueueType,
    ) -> Vec<Self> {
        match session_id {
            SessionId::Session1 => Self::new_session1_paramsets(exp_id, queue_type),
            SessionId::Session2 => Self::new_session2_paramsets(exp_id, queue_type),
            SessionId::Session2_1 => Self::new_session2_1_paramsets(exp_id, queue_type),
            SessionId::Session3 => Self::new_session3_paramsets(exp_id, queue_type),
        }
    }

    fn new_session1_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let mut start_id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &num_nodes in &[20u32, 40u32, 80u32] {
            let peering_degrees_list = &[
                vec![(num_nodes.checked_div(5).unwrap(), 1.0)],
                vec![(num_nodes.checked_div(4).unwrap(), 1.0)],
                vec![(num_nodes.checked_div(2).unwrap(), 1.0)],
            ];
            let min_queue_size_list = &[
                num_nodes.checked_div(2).unwrap().try_into().unwrap(),
                num_nodes.try_into().unwrap(),
                num_nodes.checked_mul(2).unwrap().try_into().unwrap(),
            ];
            let transmission_rate_list = &[
                num_nodes.checked_div(2).unwrap().try_into().unwrap(),
                num_nodes.try_into().unwrap(),
                num_nodes.checked_mul(2).unwrap().try_into().unwrap(),
            ];
            let num_sent_msgs_list = |_| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => vec![8, 16, 32],
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => {
                    vec![
                        num_nodes.checked_div(10).unwrap(),
                        num_nodes.checked_div(5).unwrap(),
                        num_nodes.checked_div(2).unwrap(),
                    ]
                }
            };
            let random_senders_every_time = exp_id == ExperimentId::Experiment5;
            let num_iterations = num_nodes.checked_div(2).unwrap().try_into().unwrap();

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degrees_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
                random_senders_every_time,
                queue_type,
                num_iterations,
            );
            paramsets.append(&mut new_paramsets);
            start_id = next_start_id;
        }
        paramsets
    }

    fn new_session2_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let mut start_id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &num_nodes in &[100u32, 1000u32, 10000u32] {
            let peering_degrees_list = &[vec![(4, 1.0)], vec![(8, 1.0)], vec![(16, 1.0)]];
            let min_queue_size_list = &[10, 50, 100];
            let transmission_rate_list = &[1, 10, 100];
            let num_sent_msgs_list = |min_queue_size: u16| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => {
                    vec![
                        min_queue_size.checked_div(2).unwrap().into(),
                        min_queue_size.into(),
                        min_queue_size.checked_mul(2).unwrap().into(),
                    ]
                }
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => {
                    vec![
                        num_nodes.checked_div(10).unwrap(),
                        num_nodes.checked_div(5).unwrap(),
                        num_nodes.checked_div(2).unwrap(),
                    ]
                }
            };
            let random_senders_every_time = exp_id == ExperimentId::Experiment5;
            let num_iterations = 20;

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degrees_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
                random_senders_every_time,
                queue_type,
                num_iterations,
            );
            paramsets.append(&mut new_paramsets);
            start_id = next_start_id;
        }
        paramsets
    }

    fn new_session2_1_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let mut start_id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &num_nodes in &[20u32, 200u32, 2000u32] {
            let peering_degrees_list = &[vec![(4, 1.0)], vec![(6, 1.0)], vec![(8, 1.0)]];
            let min_queue_size_list = &[10, 50, 100];
            let transmission_rate_list = &[1];
            let num_sent_msgs_list = |_| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => vec![1000],
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3
                | ExperimentId::Experiment4
                | ExperimentId::Experiment5 => {
                    vec![
                        num_nodes.checked_div(10).unwrap(),
                        num_nodes.checked_div(5).unwrap(),
                        num_nodes.checked_div(2).unwrap(),
                    ]
                }
            };
            let random_senders_every_time = exp_id == ExperimentId::Experiment5;
            let num_iterations = 20;

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degrees_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
                random_senders_every_time,
                queue_type,
                num_iterations,
            );
            paramsets.append(&mut new_paramsets);
            start_id = next_start_id;
        }
        paramsets
    }

    fn new_session3_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let start_id: u16 = 1;

        let num_nodes: u32 = 100000;
        let peering_degrees = vec![(4, 0.87), (129, 0.123), (500, 0.07)];
        let min_queue_size_list = &[10, 50, 100];
        let transmission_rate_list = &[1];
        let num_sent_msgs_list = |min_queue_size: u16| match exp_id {
            ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
            ExperimentId::Experiment2 | ExperimentId::Experiment4 | ExperimentId::Experiment5 => {
                vec![
                    min_queue_size.checked_div(2).unwrap().into(),
                    min_queue_size.into(),
                    min_queue_size.checked_mul(2).unwrap().into(),
                ]
            }
        };
        let num_senders_list = match exp_id {
            ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
            ExperimentId::Experiment3 | ExperimentId::Experiment4 | ExperimentId::Experiment5 => {
                vec![
                    num_nodes.checked_div(10).unwrap(),
                    num_nodes.checked_div(5).unwrap(),
                    num_nodes.checked_div(2).unwrap(),
                ]
            }
        };
        let random_senders_every_time = exp_id == ExperimentId::Experiment5;
        let num_iterations = 100;

        let (paramsets, _) = Self::new_paramsets(
            start_id,
            num_nodes,
            &[peering_degrees],
            min_queue_size_list,
            transmission_rate_list,
            num_sent_msgs_list,
            num_senders_list.as_slice(),
            random_senders_every_time,
            queue_type,
            num_iterations,
        );
        paramsets
    }

    #[allow(clippy::too_many_arguments)]
    fn new_paramsets(
        start_id: u16,
        num_nodes: u32,
        peering_degrees_list: &[PeeringDegreeRates],
        min_queue_size_list: &[u16],
        transmission_rate_list: &[u16],
        num_sent_msgs_list: impl Fn(u16) -> Vec<u32>,
        num_senders_list: &[u32],
        random_senders_every_time: bool,
        queue_type: QueueType,
        num_iterations: usize,
    ) -> (Vec<ParamSet>, u16) {
        let mut id = start_id;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for peering_degrees in peering_degrees_list {
            for &min_queue_size in min_queue_size_list {
                for &transmission_rate in transmission_rate_list {
                    for &num_sent_msgs in num_sent_msgs_list(min_queue_size).iter() {
                        for &num_senders in num_senders_list {
                            if !Self::is_min_queue_size_applicable(&queue_type)
                                && min_queue_size != min_queue_size_list[0]
                            {
                                id += 1;
                                continue;
                            }
                            paramsets.push(ParamSet {
                                id,
                                num_nodes,
                                peering_degree_rates: peering_degrees.clone(),
                                min_queue_size,
                                transmission_rate,
                                num_sent_msgs,
                                num_senders,
                                random_senders_every_time,
                                queue_type,
                                num_iterations,
                            });
                            id += 1;
                        }
                    }
                }
            }
        }
        (paramsets, id)
    }

    pub fn is_min_queue_size_applicable(queue_type: &QueueType) -> bool {
        matches!(
            queue_type,
            QueueType::PureCoinFlipping
                | QueueType::PureRandomSampling
                | QueueType::PermutedCoinFlipping
        )
    }

    pub fn total_num_messages(&self) -> u32 {
        self.num_sent_msgs.checked_mul(self.num_senders).unwrap()
    }

    pub fn as_csv_record(&self) -> Vec<String> {
        let peering_degrees = self
            .peering_degree_rates
            .iter()
            .map(|(degree, rate)| format!("({degree}:{rate})"))
            .collect::<Vec<String>>()
            .join(",");
        vec![
            self.id.to_string(),
            self.num_nodes.to_string(),
            format!("[{peering_degrees}]"),
            self.min_queue_size.to_string(),
            self.transmission_rate.to_string(),
            self.num_sent_msgs.to_string(),
            self.num_senders.to_string(),
            self.random_senders_every_time.to_string(),
            format!("{:?}", self.queue_type),
            self.num_iterations.to_string(),
        ]
    }

    pub fn gen_peering_degrees(&self, seed: u64) -> Vec<u32> {
        let mut vec = Vec::with_capacity(self.num_nodes as usize);
        self.peering_degree_rates.iter().for_each(|(degree, rate)| {
            let num_nodes = std::cmp::min(
                (self.num_nodes as f32 * rate).round() as u32,
                self.num_nodes - vec.len() as u32,
            );
            vec.extend(std::iter::repeat(*degree).take(num_nodes as usize));
        });
        assert_eq!(vec.len(), self.num_nodes as usize);
        vec.shuffle(&mut StdRng::seed_from_u64(seed));
        vec
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    use crate::paramset::ParamSet;

    use super::*;

    #[test]
    fn test_new_all_paramsets() {
        let cases = vec![
            (
                (ExperimentId::Experiment1, SessionId::Session1),
                3u32.pow(4),
            ),
            (
                (ExperimentId::Experiment2, SessionId::Session1),
                3u32.pow(5),
            ),
            (
                (ExperimentId::Experiment3, SessionId::Session1),
                3u32.pow(5),
            ),
            (
                (ExperimentId::Experiment4, SessionId::Session1),
                3u32.pow(6),
            ),
            (
                (ExperimentId::Experiment5, SessionId::Session1),
                3u32.pow(6),
            ),
            (
                (ExperimentId::Experiment1, SessionId::Session2),
                3u32.pow(4),
            ),
            (
                (ExperimentId::Experiment4, SessionId::Session2),
                3u32.pow(6),
            ),
            (
                (ExperimentId::Experiment5, SessionId::Session2),
                3u32.pow(6),
            ),
            (
                (ExperimentId::Experiment1, SessionId::Session2_1),
                3u32.pow(3),
            ),
            (
                (ExperimentId::Experiment4, SessionId::Session2_1),
                3u32.pow(4),
            ),
            (
                (ExperimentId::Experiment5, SessionId::Session2_1),
                3u32.pow(4),
            ),
            (
                (ExperimentId::Experiment5, SessionId::Session3),
                3u32.pow(3),
            ),
        ];

        for queue_type in QueueType::iter() {
            for ((exp_id, session_id), mut expected_cnt) in cases.clone().into_iter() {
                let paramsets = ParamSet::new_all_paramsets(exp_id, session_id, queue_type);

                // Check if the number of parameter sets is correct
                if !ParamSet::is_min_queue_size_applicable(&queue_type) {
                    expected_cnt /= 3;
                }
                assert_eq!(paramsets.len(), expected_cnt as usize);

                // Check if all parameter sets are unique
                let unique_paramsets: HashSet<Vec<String>> = paramsets
                    .iter()
                    .map(|paramset| paramset.as_csv_record())
                    .collect();
                assert_eq!(unique_paramsets.len(), paramsets.len());

                // Check if paramset IDs are correct.
                if ParamSet::is_min_queue_size_applicable(&queue_type) {
                    for (i, paramset) in paramsets.iter().enumerate() {
                        assert_eq!(paramset.id as usize, i + 1);
                    }
                }
            }
        }
    }

    #[test]
    fn test_id_consistency() {
        let cases = vec![
            (ExperimentId::Experiment1, SessionId::Session1),
            (ExperimentId::Experiment2, SessionId::Session1),
            (ExperimentId::Experiment3, SessionId::Session1),
            (ExperimentId::Experiment4, SessionId::Session1),
            (ExperimentId::Experiment5, SessionId::Session1),
            (ExperimentId::Experiment1, SessionId::Session2),
            (ExperimentId::Experiment4, SessionId::Session2),
            (ExperimentId::Experiment5, SessionId::Session2),
            (ExperimentId::Experiment1, SessionId::Session2_1),
            (ExperimentId::Experiment4, SessionId::Session2_1),
            (ExperimentId::Experiment5, SessionId::Session2_1),
            (ExperimentId::Experiment5, SessionId::Session3),
        ];

        for (exp_id, session_id) in cases.into_iter() {
            let paramsets_with_min_queue_size =
                ParamSet::new_all_paramsets(exp_id, session_id, QueueType::PureCoinFlipping);
            let paramsets_without_min_queue_size =
                ParamSet::new_all_paramsets(exp_id, session_id, QueueType::NonMix);

            for (i, paramset) in paramsets_with_min_queue_size.iter().enumerate() {
                assert_eq!(paramset.id as usize, i + 1);
            }

            for mut paramset in paramsets_without_min_queue_size.into_iter() {
                // To compare ParameterSet instances, use the same queue type.
                paramset.queue_type = QueueType::PureCoinFlipping;
                assert_eq!(
                    paramset,
                    paramsets_with_min_queue_size[paramset.id as usize - 1]
                );
            }
        }
    }
}
