use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize};

use super::{Node, NodeId};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct MixNodeState {
    pub mock_counter: usize,
}

#[derive(Debug, Clone)]
pub enum MixMessage {
    Dummy(String),
}

impl PayloadSize for MixMessage {
    fn size_bytes(&self) -> u32 {
        todo!()
    }
}

pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixNodeState,
    _settings: MixnodeSettings,
    network_interface: InMemoryNetworkInterface<MixMessage>,
}

impl MixNode {
    pub fn new(
        id: NodeId,
        settings: MixnodeSettings,
        network_interface: InMemoryNetworkInterface<MixMessage>,
    ) -> Self {
        Self {
            id,
            network_interface,
            _settings: settings,
            state: MixNodeState::default(),
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
        let _messages = self.network_interface.receive_messages();
        self.state.mock_counter += 1;
        println!(">>>>> Node {}, Step: {}", self.id, self.state.mock_counter);

        // Do stuff on the messages;
        // Network interface can be passed into the functions for outputting the messages:
        // ```rust
        // self.network_interface.send_message(receiving_node_id, payload);
        // ```
    }
}
