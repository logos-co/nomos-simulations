use crate::queue::QueueType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExperimentId {
    Experiment1 = 1,
    Experiment2 = 2,
    Experiment3 = 3,
    Experiment4 = 4,
}

impl std::str::FromStr for ExperimentId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Experiment1" => Ok(ExperimentId::Experiment1),
            "2" | "Experiment2" => Ok(ExperimentId::Experiment2),
            "3" | "Experiment3" => Ok(ExperimentId::Experiment3),
            "4" | "Experiment4" => Ok(ExperimentId::Experiment4),
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
}

impl std::str::FromStr for SessionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Session1" => Ok(SessionId::Session1),
            "2" | "Session2" => Ok(SessionId::Session2),
            "2.1" | "Session21" => Ok(SessionId::Session2_1),
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
    "queue_type",
    "num_iterations",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamSet {
    pub id: u16,
    pub num_nodes: u16,
    pub peering_degree: u16,
    pub min_queue_size: u16,
    pub transmission_rate: u16,
    pub num_sent_msgs: u16,
    pub num_senders: u16,
    pub queue_type: QueueType,
    pub num_iterations: u16,
}

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
        }
    }

    fn new_session1_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let mut start_id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &num_nodes in &[20, 40, 80] {
            let peering_degree_list = &[num_nodes / 5, num_nodes / 4, num_nodes / 2];
            let min_queue_size_list = &[num_nodes / 2, num_nodes, num_nodes * 2];
            let transmission_rate_list = &[num_nodes / 2, num_nodes, num_nodes * 2];
            let num_sent_msgs_list = |_| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2 | ExperimentId::Experiment4 => vec![8, 16, 32],
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3 | ExperimentId::Experiment4 => {
                    vec![num_nodes / 10, num_nodes / 5, num_nodes / 2]
                }
            };
            let num_iterations = num_nodes / 2;

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degree_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
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
        for &num_nodes in &[100, 1000, 10000] {
            let peering_degree_list = &[4, 8, 16];
            let min_queue_size_list = &[10, 50, 100];
            let transmission_rate_list = &[1, 10, 100];
            let num_sent_msgs_list = |min_queue_size| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2 | ExperimentId::Experiment4 => {
                    vec![min_queue_size / 2, min_queue_size, min_queue_size * 2]
                }
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3 | ExperimentId::Experiment4 => {
                    vec![num_nodes / 10, num_nodes / 5, num_nodes / 2]
                }
            };
            let num_iterations = 20;

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degree_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
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
        for &num_nodes in &[20, 200, 2000] {
            let peering_degree_list = &[4, 6, 8];
            let min_queue_size_list = &[10, 50, 100];
            let transmission_rate_list = &[1];
            let num_sent_msgs_list = |_| match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment3 => vec![1],
                ExperimentId::Experiment2 | ExperimentId::Experiment4 => vec![1000],
            };
            let num_senders_list = match exp_id {
                ExperimentId::Experiment1 | ExperimentId::Experiment2 => vec![1],
                ExperimentId::Experiment3 | ExperimentId::Experiment4 => {
                    vec![num_nodes / 10, num_nodes / 5, num_nodes / 2]
                }
            };
            let num_iterations = 20;

            let (mut new_paramsets, next_start_id) = Self::new_paramsets(
                start_id,
                num_nodes,
                peering_degree_list,
                min_queue_size_list,
                transmission_rate_list,
                num_sent_msgs_list,
                num_senders_list.as_slice(),
                queue_type,
                num_iterations,
            );
            paramsets.append(&mut new_paramsets);
            start_id = next_start_id;
        }
        paramsets
    }

    #[allow(clippy::too_many_arguments)]
    fn new_paramsets(
        start_id: u16,
        num_nodes: u16,
        peering_degree_list: &[u16],
        min_queue_size_list: &[u16],
        transmission_rate_list: &[u16],
        num_sent_msgs_list: impl Fn(u16) -> Vec<u16>,
        num_senders_list: &[u16],
        queue_type: QueueType,
        num_iterations: u16,
    ) -> (Vec<ParamSet>, u16) {
        let mut id = start_id;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &peering_degree in peering_degree_list {
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
                                peering_degree,
                                min_queue_size,
                                transmission_rate,
                                num_sent_msgs,
                                num_senders,
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

    pub fn as_csv_record(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.num_nodes.to_string(),
            self.peering_degree.to_string(),
            self.min_queue_size.to_string(),
            self.transmission_rate.to_string(),
            self.num_sent_msgs.to_string(),
            self.num_senders.to_string(),
            format!("{:?}", self.queue_type),
            self.num_iterations.to_string(),
        ]
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
                (ExperimentId::Experiment1, SessionId::Session2),
                3u32.pow(4),
            ),
            (
                (ExperimentId::Experiment4, SessionId::Session2),
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
                let unique_paramsets: HashSet<ParamSet> = paramsets.clone().into_iter().collect();
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
            (ExperimentId::Experiment1, SessionId::Session2),
            (ExperimentId::Experiment4, SessionId::Session2),
            (ExperimentId::Experiment1, SessionId::Session2_1),
            (ExperimentId::Experiment4, SessionId::Session2_1),
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
