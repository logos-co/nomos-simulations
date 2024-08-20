use queue::QueueType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExperimentId {
    Experiment1 = 1,
    Experiment2 = 2,
}

impl std::str::FromStr for ExperimentId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Experiment1" => Ok(ExperimentId::Experiment1),
            "2" | "Experiment2" => Ok(ExperimentId::Experiment2),
            _ => Err(format!("Invalid experiment ID: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SessionId {
    Session1 = 1,
}

impl std::str::FromStr for SessionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" | "Session1" => Ok(SessionId::Session1),
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
    "num_sender_data_msgs",
    "sender_data_msg_prob",
    "mix_data_msg_prob",
    "queue_type",
    "num_iterations",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParamSet {
    pub id: u16,
    pub num_nodes: u32,
    pub peering_degree: u32,
    pub min_queue_size: u16,
    pub transmission_rate: u16,
    pub num_sender_data_msgs: u32,
    pub sender_data_msg_prob: f32,
    pub mix_data_msg_prob: f32,
    pub queue_type: QueueType,
    pub num_iterations: usize,
}

impl ParamSet {
    pub fn new_all_paramsets(
        exp_id: ExperimentId,
        session_id: SessionId,
        queue_type: QueueType,
    ) -> Vec<Self> {
        match session_id {
            SessionId::Session1 => Self::new_session1_paramsets(exp_id, queue_type),
        }
    }

    fn new_session1_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let num_nodes: u32 = 3;
        let peering_degree: u32 = 1;
        let transmission_rate: u16 = 1;
        let min_queue_sizes: &[u16] = &[
            transmission_rate.checked_div(2).unwrap(),
            transmission_rate,
            transmission_rate.checked_mul(2).unwrap(),
        ];
        let num_sender_data_msgs: u32 = (transmission_rate as u32).checked_mul(1000).unwrap();
        let sender_data_msg_probs: &[f32] = &[0.01, 0.1, 0.5, 0.9, 0.99, 1.0];
        let mix_data_msg_probs: &[f32] = match exp_id {
            ExperimentId::Experiment1 => &[0.0],
            ExperimentId::Experiment2 => &[0.00001, 0.0001, 0.001, 0.01, 0.1],
        };
        let num_iterations: usize = 100;

        let mut id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        for &min_queue_size in min_queue_sizes {
            for &sender_data_msg_prob in sender_data_msg_probs {
                for &mix_data_msg_prob in mix_data_msg_probs {
                    if !Self::is_min_queue_size_applicable(&queue_type)
                        && min_queue_size != min_queue_sizes[0]
                    {
                        id += 1;
                        continue;
                    }
                    let paramset = ParamSet {
                        id,
                        num_nodes,
                        peering_degree,
                        min_queue_size,
                        transmission_rate,
                        num_sender_data_msgs,
                        sender_data_msg_prob,
                        mix_data_msg_prob,
                        queue_type,
                        num_iterations,
                    };
                    id += 1;
                    paramsets.push(paramset);
                }
            }
        }
        paramsets
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
            self.num_sender_data_msgs.to_string(),
            self.sender_data_msg_prob.to_string(),
            self.mix_data_msg_prob.to_string(),
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
            ((ExperimentId::Experiment1, SessionId::Session1), 3 * 6),
            ((ExperimentId::Experiment2, SessionId::Session1), 3 * 6 * 5),
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
