mod step_scheduler;

use super::{Node, NodeId};
use crate::network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
        2208
    }
}

pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixNodeState,
    settings: MixnodeSettings,
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
            settings,
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
        let messages = self.network_interface.receive_messages();
        for message in messages {
            println!(">>>>> Node {}, message: {message:?}", self.id);
        }

        self.state.mock_counter += 1;

        for node_id in self.settings.connected_peers.iter() {
            self.network_interface.send_message(
                *node_id,
                MixMessage::Dummy(format!("Hello from node: {}", self.id)),
            )
        }
    }
}
