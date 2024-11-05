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

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode<S> {
    id: NodeId,
    state: MixNodeState,
    #[allow(dead_code)]
    settings: S,
}

impl<S: Send + Sync> MixNode<S> {
    pub fn new(id: NodeId, settings: S) -> Self {
        Self {
            id,
            state: MixNodeState::default(),
            settings,
        }
    }
}

impl<S> Node for MixNode<S>
where
    S: Send + Sync,
{
    type Settings = S;

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
