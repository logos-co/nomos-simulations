use std::fmt::Display;

use protocol::queue::QueueType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExperimentId {
    Experiment1 = 1,
    Experiment2 = 2,
    Experiment3 = 3,
    Experiment4 = 4,
    Experiment5 = 5,
    Experiment6 = 6,
}

impl std::str::FromStr for ExperimentId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(ExperimentId::Experiment1),
            "2" => Ok(ExperimentId::Experiment2),
            "3" => Ok(ExperimentId::Experiment3),
            "4" => Ok(ExperimentId::Experiment4),
            "5" => Ok(ExperimentId::Experiment5),
            "6" => Ok(ExperimentId::Experiment6),
            _ => Err(format!("Invalid experiment ID: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SessionId {
    Session1 = 1,
    Session3 = 3,
}

impl std::str::FromStr for SessionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(SessionId::Session1),
            "3" => Ok(SessionId::Session3),
            _ => Err(format!("Invalid session ID: {}", s)),
        }
    }
}

pub const PARAMSET_CSV_COLUMNS: &[&str] = &[
    "paramset",
    "num_mixes",
    "num_paths",
    "random_topology",
    "peering_degree",
    "min_queue_size",
    "transmission_rate",
    "num_senders",
    "num_sender_msgs",
    "sender_data_msg_prob",
    "mix_data_msg_prob",
    "queue_type",
    "num_iterations",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParamSet {
    pub id: u16,
    pub num_mixes: u32,
    pub num_paths: u16,
    pub random_topology: bool,
    pub peering_degree: PeeringDegree,
    pub min_queue_size: u16,
    pub transmission_rate: u16,
    pub num_senders: u8,
    pub num_sender_msgs: u32,
    pub sender_data_msg_prob: f32,
    pub mix_data_msg_prob: f32,
    pub queue_type: QueueType,
    pub num_iterations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeeringDegree {
    Fixed(u32),
    Random(Vec<(u32, f32)>),
}

impl Display for PeeringDegree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeeringDegree::Fixed(c) => write!(f, "{c}"),
            PeeringDegree::Random(c_probs) => write!(f, "{c_probs:?}"),
        }
    }
}

impl ParamSet {
    pub fn new_all_paramsets(
        exp_id: ExperimentId,
        session_id: SessionId,
        queue_type: QueueType,
    ) -> Vec<Self> {
        match session_id {
            SessionId::Session1 => Self::new_session1_paramsets(exp_id, queue_type),
            SessionId::Session3 => Self::new_session3_paramsets(exp_id, queue_type),
        }
    }

    fn new_session1_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let transmission_rate: u16 = 1;
        let min_queue_size: u16 = 10;
        let num_senders: u8 = match exp_id {
            ExperimentId::Experiment3 | ExperimentId::Experiment4 => 2,
            _ => 1,
        };
        let num_sender_msgs: u32 = match exp_id {
            ExperimentId::Experiment6 => 10000,
            _ => 1000000,
        };
        let sender_data_msg_probs: &[f32] = match exp_id {
            ExperimentId::Experiment6 => &[0.01, 0.1, 0.5],
            _ => &[0.01, 0.1, 0.5, 0.9, 0.99, 1.0],
        };
        let mix_data_msg_probs = |num_mixes: u32| match exp_id {
            ExperimentId::Experiment1 | ExperimentId::Experiment3 | ExperimentId::Experiment5 => {
                vec![0.0]
            }
            ExperimentId::Experiment2 | ExperimentId::Experiment4 => vec![0.001, 0.01, 0.1],
            ExperimentId::Experiment6 => {
                let g: f32 = num_mixes as f32;
                vec![1.0 / (2.0 * g), 1.0 / g, 2.0 / g]
            }
        };

        let mut id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        match exp_id {
            ExperimentId::Experiment1
            | ExperimentId::Experiment2
            | ExperimentId::Experiment3
            | ExperimentId::Experiment4 => {
                for &num_paths in &[1, 2, 3, 4] {
                    for &num_mixes in &[1, 2, 3, 4] {
                        for &sender_data_msg_prob in sender_data_msg_probs {
                            for &mix_data_msg_prob in &mix_data_msg_probs(num_mixes) {
                                let paramset = ParamSet {
                                    id,
                                    num_mixes,
                                    num_paths,
                                    random_topology: false,
                                    peering_degree: PeeringDegree::Fixed(1),
                                    min_queue_size,
                                    transmission_rate,
                                    num_senders,
                                    num_sender_msgs,
                                    sender_data_msg_prob,
                                    mix_data_msg_prob,
                                    queue_type,
                                    num_iterations: 1,
                                };
                                id += 1;
                                paramsets.push(paramset);
                            }
                        }
                    }
                }
            }
            ExperimentId::Experiment5 | ExperimentId::Experiment6 => {
                for &num_mixes in &[8, 16, 32] {
                    for &peering_degree in &[2, 3, 4] {
                        for &sender_data_msg_prob in sender_data_msg_probs {
                            for &mix_data_msg_prob in &mix_data_msg_probs(num_mixes) {
                                let paramset = ParamSet {
                                    id,
                                    num_mixes,
                                    num_paths: 0, // since we're gonna build random topology
                                    random_topology: true,
                                    peering_degree: PeeringDegree::Fixed(peering_degree),
                                    min_queue_size,
                                    transmission_rate,
                                    num_senders,
                                    num_sender_msgs,
                                    sender_data_msg_prob,
                                    mix_data_msg_prob,
                                    queue_type,
                                    num_iterations: 10,
                                };
                                id += 1;
                                paramsets.push(paramset);
                            }
                        }
                    }
                }
            }
        }

        paramsets
    }

    fn new_session3_paramsets(exp_id: ExperimentId, queue_type: QueueType) -> Vec<ParamSet> {
        let sender_data_msg_probs: &[f32] = match exp_id {
            ExperimentId::Experiment5 => &[0.01, 0.1, 0.5, 0.9, 0.99, 1.0],
            ExperimentId::Experiment6 => &[0.01, 0.1, 0.5],
            _ => {
                panic!("Only Experiment5 and Experiment6 are supported for Session3");
            }
        };
        let mix_data_msg_probs = |num_mixes: u32| match exp_id {
            ExperimentId::Experiment5 => {
                vec![0.0]
            }
            ExperimentId::Experiment6 => {
                let g: f32 = num_mixes as f32;
                vec![1.0 / (2.0 * g), 1.0 / g, 2.0 / g]
            }
            _ => {
                panic!("Only Experiment5 and Experiment6 are supported for Session3");
            }
        };

        let mut id: u16 = 1;
        let mut paramsets: Vec<ParamSet> = Vec::new();
        match exp_id {
            ExperimentId::Experiment5 | ExperimentId::Experiment6 => {
                let num_mixes = 32;
                for &sender_data_msg_prob in sender_data_msg_probs {
                    for &mix_data_msg_prob in &mix_data_msg_probs(num_mixes) {
                        let paramset = ParamSet {
                            id,
                            num_mixes,
                            num_paths: 0, // since we're gonna build random topology
                            random_topology: true,
                            peering_degree: PeeringDegree::Random(vec![
                                (4, 0.87),
                                (12, 0.123),
                                (24, 0.007),
                            ]),
                            min_queue_size: 10,
                            transmission_rate: 1,
                            num_senders: 1,
                            num_sender_msgs: match exp_id {
                                ExperimentId::Experiment6 => 10000,
                                _ => 1000000,
                            },
                            sender_data_msg_prob,
                            mix_data_msg_prob,
                            queue_type,
                            num_iterations: 10,
                        };
                        id += 1;
                        paramsets.push(paramset);
                    }
                }
            }
            _ => {
                panic!("Only Experiment5 and Experiment6 are supported for Session3");
            }
        }

        paramsets
    }

    pub fn num_sender_or_receiver_conns(&self) -> usize {
        if self.random_topology {
            match &self.peering_degree {
                PeeringDegree::Fixed(c) => *c as usize,
                PeeringDegree::Random(c_probs) => {
                    c_probs.iter().map(|(c, _)| *c).min().unwrap() as usize
                }
            }
        } else {
            self.num_paths as usize
        }
    }

    pub fn as_csv_record(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.num_mixes.to_string(),
            self.num_paths.to_string(),
            self.random_topology.to_string(),
            self.peering_degree.to_string(),
            self.min_queue_size.to_string(),
            self.transmission_rate.to_string(),
            self.num_senders.to_string(),
            self.num_sender_msgs.to_string(),
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
            ((ExperimentId::Experiment1, SessionId::Session1), 4 * 4 * 6),
            (
                (ExperimentId::Experiment2, SessionId::Session1),
                4 * 4 * 6 * 3,
            ),
            ((ExperimentId::Experiment3, SessionId::Session1), 4 * 4 * 6),
            (
                (ExperimentId::Experiment4, SessionId::Session1),
                4 * 4 * 6 * 3,
            ),
            ((ExperimentId::Experiment5, SessionId::Session1), 3 * 3 * 6),
            (
                (ExperimentId::Experiment6, SessionId::Session1),
                3 * 3 * 3 * 3,
            ),
            ((ExperimentId::Experiment5, SessionId::Session3), 6),
            ((ExperimentId::Experiment6, SessionId::Session3), 3 * 3),
        ];

        for queue_type in QueueType::iter() {
            for ((exp_id, session_id), expected_cnt) in cases.clone().into_iter() {
                let paramsets = ParamSet::new_all_paramsets(exp_id, session_id, queue_type);

                assert_eq!(
                    paramsets.len(),
                    expected_cnt as usize,
                    "queue_type:{:?}, exp:{:?}, session:{:?}",
                    queue_type,
                    exp_id,
                    session_id,
                );

                // Check if all parameter sets are unique
                let unique_paramsets: HashSet<Vec<String>> = paramsets
                    .iter()
                    .map(|paramset| paramset.as_csv_record())
                    .collect();
                assert_eq!(unique_paramsets.len(), paramsets.len());

                // Check if paramset IDs are correct.
                for (i, paramset) in paramsets.iter().enumerate() {
                    assert_eq!(paramset.id as usize, i + 1);
                    println!("{:?}", paramset);
                }

                // Check PeeringDegree
                for paramset in paramsets.iter() {
                    match session_id {
                        SessionId::Session1 => {
                            assert!(matches!(paramset.peering_degree, PeeringDegree::Fixed(_)))
                        }
                        SessionId::Session3 => {
                            assert!(matches!(paramset.peering_degree, PeeringDegree::Random(_)))
                        }
                    }
                }
            }
        }
    }
}
