use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{Node, NodeId};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct MixNodeState {
    pub mock_counter: usize,
}

#[derive(Debug, Clone)]
pub enum MixMessage {
    Dummy(String),
}

pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixNodeState,
    settings: MixnodeSettings,
}

impl MixNode {
    pub fn new(id: NodeId, settings: MixnodeSettings) -> Self {
        Self {
            id,
            state: MixNodeState::default(),
            settings,
        }
    }
}

impl Node for MixNode {
    type Settings = MixnodeSettings;

    type State = MixNodeState;

    fn id(&self) -> NodeId {
        self.id
    }

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn step(&mut self, _: Duration) {
        todo!()
    }
}
