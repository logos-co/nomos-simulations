use std::any::Any;

use serde::Serialize;

use crate::{
    node::NodeId,
    output_processors::{Record, RecordType, Runtime},
    settings::SimulationSettings,
    warding::SimulationState,
};

#[derive(Debug, Clone, Serialize)]
pub struct MixnodeState {
    pub node_id: NodeId,
    pub mock_counter: usize,
    pub step_id: usize,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MixnodeRecord {
    Runtime(Runtime),
    Settings(Box<SimulationSettings>),
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
