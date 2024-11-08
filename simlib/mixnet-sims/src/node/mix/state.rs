use std::{any::Any, collections::HashMap};

use serde::Serialize;

use netrunner::{
    node::{serialize_node_id_as_index, NodeId},
    output_processors::{Record, RecordType, Runtime},
    settings::SimulationSettings,
    warding::SimulationState,
};

use super::message::PayloadId;

#[derive(Debug, Clone, Serialize)]
pub struct MixnodeState {
    #[serde(serialize_with = "serialize_node_id_as_index")]
    pub node_id: NodeId,
    pub step_id: usize,
    // For latency measurement
    pub data_messages_generated: HashMap<PayloadId, usize>,
    pub data_messages_fully_unwrapped: HashMap<PayloadId, usize>,
    // For anonymity measurement
    pub accum_num_inbound_messages: usize,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MixnodeRecord {
    Runtime(Runtime),
    Settings(Box<SimulationSettings>),
    #[allow(clippy::vec_box)] // we downcast stuff and we need the extra boxing
    Data(Vec<Box<MixnodeState>>),
}

impl From<Runtime> for MixnodeRecord {
    fn from(value: Runtime) -> Self {
        Self::Runtime(value)
    }
}

impl From<SimulationSettings> for MixnodeRecord {
    fn from(value: SimulationSettings) -> Self {
        Self::Settings(Box::new(value))
    }
}

impl Record for MixnodeRecord {
    type Data = MixnodeState;

    fn record_type(&self) -> RecordType {
        match self {
            MixnodeRecord::Runtime(_) => RecordType::Meta,
            MixnodeRecord::Settings(_) => RecordType::Settings,
            MixnodeRecord::Data(_) => RecordType::Data,
        }
    }

    fn data(&self) -> Vec<&MixnodeState> {
        match self {
            MixnodeRecord::Data(d) => d.iter().map(AsRef::as_ref).collect(),
            _ => vec![],
        }
    }
}

impl<S, T: Clone + Serialize + 'static> TryFrom<&SimulationState<S, T>> for MixnodeRecord {
    type Error = anyhow::Error;

    fn try_from(state: &SimulationState<S, T>) -> Result<Self, Self::Error> {
        let Ok(states) = state
            .nodes
            .read()
            .iter()
            .map(|n| Box::<dyn Any + 'static>::downcast(Box::new(n.state().clone())))
            .collect::<Result<Vec<_>, _>>()
        else {
            return Err(anyhow::anyhow!("use carnot record on other node"));
        };
        Ok(Self::Data(states))
    }
}
