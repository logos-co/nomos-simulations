pub mod state;

use super::{Node, NodeId};
use crate::network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize};
use serde::Deserialize;
use state::MixnodeState;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum MixMessage {
    Dummy(String),
}

impl PayloadSize for MixMessage {
    fn size_bytes(&self) -> u32 {
        2208
    }
}

#[derive(Clone, Default, Deserialize)]
pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixnodeState,
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
            state: MixnodeState {
                node_id: id,
                mock_counter: 0,
                step_id: 0,
            },
        }
    }
}

impl Node for MixNode {
    type Settings = MixnodeSettings;

    type State = MixnodeState;

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

        self.state.step_id += 1;
        self.state.mock_counter += 1;

        for node_id in self.settings.connected_peers.iter() {
            self.network_interface.send_message(
                *node_id,
                MixMessage::Dummy(format!("Hello from node: {}", self.id)),
            )
        }
    }
}
