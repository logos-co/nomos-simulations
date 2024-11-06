pub mod state;
mod step_scheduler;
mod stream_wrapper;

use super::{Node, NodeId};
use crate::network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize};
use crossbeam::channel;
use futures::Stream;
use nomos_mix::persistent_transmission::{
    PersistentTransmissionExt, PersistentTransmissionSettings, PersistentTransmissionStream,
};
use nomos_mix_message::mock::MockMixMessage;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde::Deserialize;
use state::MixnodeState;
use std::{
    pin::{self},
    task::Poll,
    time::Duration,
};
use step_scheduler::Interval;
use stream_wrapper::CrossbeamReceiverStream;

#[derive(Debug, Clone)]
pub enum MixMessage {
    Dummy(Vec<u8>),
}

impl PayloadSize for MixMessage {
    fn size_bytes(&self) -> u32 {
        2208
    }
}

#[derive(Clone, Default, Deserialize)]
pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
    pub seed: u64,
    pub persistent_transmission: PersistentTransmissionSettings,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixnodeState,
    settings: MixnodeSettings,
    network_interface: InMemoryNetworkInterface<MixMessage>,

    persistent_sender: channel::Sender<Vec<u8>>,
    update_time_sender: channel::Sender<Duration>,
    persistent_transmission_messages: PersistentTransmissionStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        MockMixMessage,
        Interval,
    >,
}

impl MixNode {
    pub fn new(
        id: NodeId,
        settings: MixnodeSettings,
        network_interface: InMemoryNetworkInterface<MixMessage>,
    ) -> Self {
        let state = MixnodeState {
            node_id: id,
            mock_counter: 0,
            step_id: 0,
        };

        let (persistent_sender, persistent_receiver) = channel::unbounded();
        let (update_time_sender, update_time_receiver) = channel::unbounded();
        let persistent_transmission_messages = CrossbeamReceiverStream::new(persistent_receiver)
            .persistent_transmission(
                settings.persistent_transmission,
                ChaCha12Rng::seed_from_u64(settings.seed),
                Interval::new(
                    Duration::from_secs_f64(
                        1.0 / settings.persistent_transmission.max_emission_frequency,
                    ),
                    update_time_receiver,
                ),
            );

        Self {
            id,
            network_interface,
            settings,
            state,
            persistent_sender,
            update_time_sender,
            persistent_transmission_messages,
        }
    }

    fn forward(&self, message: MixMessage) {
        for node_id in self.settings.connected_peers.iter() {
            self.network_interface
                .send_message(*node_id, message.clone())
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

    fn step(&mut self, elapsed: Duration) {
        let Self {
            update_time_sender,
            persistent_transmission_messages,
            ..
        } = self;

        let messages = self.network_interface.receive_messages();
        for message in messages {
            println!(">>>>> Node {}, message: {message:?}", self.id);
            let MixMessage::Dummy(msg) = message.into_payload();
            self.persistent_sender.send(msg).unwrap();
        }

        self.state.step_id += 1;
        self.state.mock_counter += 1;

        update_time_sender.send(elapsed).unwrap();

        let waker = futures::task::noop_waker();
        let mut cx = futures::task::Context::from_waker(&waker);
        if let Poll::Ready(Some(msg)) =
            pin::pin!(persistent_transmission_messages).poll_next(&mut cx)
        {
            self.forward(MixMessage::Dummy(msg));
        }
    }
}
