mod scheduler;
pub mod state;
mod stream_wrapper;

use super::{Node, NodeId};
use crate::network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize};
use crossbeam::channel;
use futures::Stream;
use multiaddr::Multiaddr;
use nomos_mix::{
    membership::Membership,
    message_blend::{MessageBlendExt, MessageBlendSettings, MessageBlendStream},
    persistent_transmission::{
        PersistentTransmissionExt, PersistentTransmissionSettings, PersistentTransmissionStream,
    },
    MixOutgoingMessage,
};
use nomos_mix_message::mock::MockMixMessage;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use scheduler::{Interval, TemporalRelease};
use serde::Deserialize;
use state::MixnodeState;
use std::{
    pin::{self},
    task::Poll,
    time::Duration,
};
use stream_wrapper::CrossbeamReceiverStream;

#[derive(Debug, Clone)]
pub struct MixMessage(Vec<u8>);

impl PayloadSize for MixMessage {
    fn size_bytes(&self) -> u32 {
        2208
    }
}

#[derive(Clone, Deserialize)]
pub struct MixnodeSettings {
    pub connected_peers: Vec<NodeId>,
    pub seed: u64,
    pub persistent_transmission: PersistentTransmissionSettings,
    pub message_blend: MessageBlendSettings<MockMixMessage>,
    pub membership: Vec<<MockMixMessage as nomos_mix_message::MixMessage>::PublicKey>,
}

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixnodeState,
    settings: MixnodeSettings,
    network_interface: InMemoryNetworkInterface<MixMessage>,

    persistent_sender: channel::Sender<Vec<u8>>,
    persistent_update_time_sender: channel::Sender<Duration>,
    persistent_transmission_messages: PersistentTransmissionStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        MockMixMessage,
        Interval,
    >,
    blend_sender: channel::Sender<Vec<u8>>,
    blend_update_time_sender: channel::Sender<Duration>,
    blend_messages: MessageBlendStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        MockMixMessage,
        TemporalRelease,
    >,
}

impl MixNode {
    pub fn new(
        id: NodeId,
        settings: MixnodeSettings,
        network_interface: InMemoryNetworkInterface<MixMessage>,
    ) -> Self {
        let mut rng_generator = ChaCha12Rng::seed_from_u64(settings.seed);

        // Init Tier-1: Persistent transmission
        let (persistent_sender, persistent_receiver) = channel::unbounded();
        let (persistent_update_time_sender, persistent_update_time_receiver) = channel::unbounded();
        let persistent_transmission_messages = CrossbeamReceiverStream::new(persistent_receiver)
            .persistent_transmission(
                settings.persistent_transmission,
                ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
                Interval::new(
                    Duration::from_secs_f64(
                        1.0 / settings.persistent_transmission.max_emission_frequency,
                    ),
                    persistent_update_time_receiver,
                ),
            );

        // Init Tier-2: message blend
        let (blend_sender, blend_receiver) = channel::unbounded();
        let (blend_update_time_sender, blend_update_time_receiver) = channel::unbounded();
        let nodes: Vec<
            nomos_mix::membership::Node<
                <MockMixMessage as nomos_mix_message::MixMessage>::PublicKey,
            >,
        > = settings
            .membership
            .iter()
            .map(|&public_key| nomos_mix::membership::Node {
                address: Multiaddr::empty(),
                public_key,
            })
            .collect();
        let membership = Membership::<MockMixMessage>::new(nodes, id.into());
        let temporal_release = TemporalRelease::new(
            ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
            blend_update_time_receiver,
            (
                1,
                settings.message_blend.temporal_processor.max_delay_seconds,
            ),
        );
        let blend_messages = CrossbeamReceiverStream::new(blend_receiver).blend(
            settings.message_blend.clone(),
            membership,
            temporal_release,
            ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
        );

        Self {
            id,
            network_interface,
            settings,
            state: MixnodeState {
                node_id: id,
                mock_counter: 0,
                step_id: 0,
                num_messages_broadcasted: 0,
            },
            persistent_sender,
            persistent_update_time_sender,
            persistent_transmission_messages,
            blend_sender,
            blend_update_time_sender,
            blend_messages,
        }
    }

    fn forward(&self, message: MixMessage) {
        for node_id in self.settings.connected_peers.iter() {
            self.network_interface
                .send_message(*node_id, message.clone())
        }
    }

    fn update_time(&mut self, elapsed: Duration) {
        self.persistent_update_time_sender.send(elapsed).unwrap();
        self.blend_update_time_sender.send(elapsed).unwrap();
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
        self.update_time(elapsed);

        let Self {
            persistent_sender,
            persistent_transmission_messages,
            blend_sender,
            blend_messages,
            ..
        } = self;

        let messages = self.network_interface.receive_messages();
        for message in messages {
            println!(">>>>> Node {}, message: {message:?}", self.id);
            blend_sender.send(message.into_payload().0).unwrap();
        }

        let waker = futures::task::noop_waker();
        let mut cx = futures::task::Context::from_waker(&waker);
        // Proceed message blend
        if let Poll::Ready(Some(msg)) = pin::pin!(blend_messages).poll_next(&mut cx) {
            match msg {
                MixOutgoingMessage::Outbound(msg) => {
                    persistent_sender.send(msg).unwrap();
                }
                MixOutgoingMessage::FullyUnwrapped(_) => {
                    self.state.num_messages_broadcasted += 1;
                    //TODO: create a tracing event
                }
            }
        }
        // Proceed persistent transmission
        if let Poll::Ready(Some(msg)) =
            pin::pin!(persistent_transmission_messages).poll_next(&mut cx)
        {
            self.forward(MixMessage(msg));
        }

        self.state.step_id += 1;
        self.state.mock_counter += 1;
    }
}
