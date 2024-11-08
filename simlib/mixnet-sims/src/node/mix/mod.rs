mod consensus_streams;
mod lottery;
mod scheduler;
pub mod state;
pub mod stream_wrapper;

use crossbeam::channel;
use futures::Stream;
use lottery::StakeLottery;
use multiaddr::Multiaddr;
use netrunner::node::{Node, NodeId};
use netrunner::{
    network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize},
    warding::WardCondition,
};
use nomos_mix::{
    membership::Membership,
    message_blend::{
        crypto::CryptographicProcessor, MessageBlendExt, MessageBlendSettings, MessageBlendStream,
    },
    persistent_transmission::{
        PersistentTransmissionExt, PersistentTransmissionSettings, PersistentTransmissionStream,
    },
    MixOutgoingMessage,
};
use nomos_mix_message::mock::MockMixMessage;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use scheduler::{Interval, TemporalRelease};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use state::MixnodeState;
use std::collections::HashSet;
use std::{pin::pin, task::Poll, time::Duration};
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
    pub data_message_lottery_interval: Duration,
    pub stake_proportion: f64,
    pub seed: u64,
    pub persistent_transmission: PersistentTransmissionSettings,
    pub message_blend: MessageBlendSettings<MockMixMessage>,
    pub membership: Vec<<MockMixMessage as nomos_mix_message::MixMessage>::PublicKey>,
}

type Sha256Hash = [u8; 32];

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixnodeState,
    settings: MixnodeSettings,
    network_interface: InMemoryNetworkInterface<MixMessage>,
    message_cache: HashSet<Sha256Hash>,

    data_msg_lottery_update_time_sender: channel::Sender<Duration>,
    data_msg_lottery_interval: Interval,
    data_msg_lottery: StakeLottery<ChaCha12Rng>,

    persistent_sender: channel::Sender<Vec<u8>>,
    persistent_update_time_sender: channel::Sender<Duration>,
    persistent_transmission_messages: PersistentTransmissionStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        MockMixMessage,
        Interval,
    >,
    crypto_processor: CryptographicProcessor<ChaCha12Rng, MockMixMessage>,
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

        // Init Interval for data message lottery
        let (data_msg_lottery_update_time_sender, data_msg_lottery_update_time_receiver) =
            channel::unbounded();
        let data_msg_lottery_interval = Interval::new(
            settings.data_message_lottery_interval,
            data_msg_lottery_update_time_receiver,
        );
        let data_msg_lottery = StakeLottery::new(
            ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
            settings.stake_proportion,
        );

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
        let crypto_processor = CryptographicProcessor::new(
            settings.message_blend.cryptographic_processor.clone(),
            membership.clone(),
            ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
        );
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
            message_cache: HashSet::new(),
            settings,
            state: MixnodeState {
                node_id: id,
                step_id: 0,
                num_messages_broadcasted: 0,
            },
            data_msg_lottery_update_time_sender,
            data_msg_lottery_interval,
            data_msg_lottery,
            persistent_sender,
            persistent_update_time_sender,
            persistent_transmission_messages,
            crypto_processor,
            blend_sender,
            blend_update_time_sender,
            blend_messages,
        }
    }

    fn forward(&mut self, message: MixMessage) {
        if !self.message_cache.insert(Self::sha256(&message.0)) {
            return;
        }
        for node_id in self.settings.connected_peers.iter() {
            self.network_interface
                .send_message(*node_id, message.clone())
        }
    }

    fn receive(&mut self) -> Vec<MixMessage> {
        self.network_interface
            .receive_messages()
            .into_iter()
            .map(|msg| msg.into_payload())
            // Retain only messages that have not been seen before
            .filter(|msg| self.message_cache.insert(Self::sha256(&msg.0)))
            .collect()
    }

    fn sha256(message: &[u8]) -> Sha256Hash {
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.finalize().into()
    }

    fn update_time(&mut self, elapsed: Duration) {
        self.data_msg_lottery_update_time_sender
            .send(elapsed)
            .unwrap();
        self.persistent_update_time_sender.send(elapsed).unwrap();
        self.blend_update_time_sender.send(elapsed).unwrap();
    }

    fn build_message_payload(&self) -> Vec<u8> {
        let payload = MessagePayload {
            node_id: self.id.into(),
            step_id: self.state.step_id,
        };
        bincode::serialize(&payload).unwrap()
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

        let waker = futures::task::noop_waker();
        let mut cx = futures::task::Context::from_waker(&waker);

        if let Poll::Ready(Some(_)) = pin!(&mut self.data_msg_lottery_interval).poll_next(&mut cx) {
            if self.data_msg_lottery.run() {
                let payload = self.build_message_payload();
                let message = self.crypto_processor.wrap_message(&payload).unwrap();
                self.persistent_sender.send(message).unwrap();
            }
        }

        // TODO: Generate cover message with probability

        for message in self.receive() {
            // println!(">>>>> Node {}, message: {message:?}", self.id);
            self.forward(message.clone());
            self.blend_sender.send(message.0).unwrap();
        }

        // Proceed message blend
        if let Poll::Ready(Some(msg)) = pin!(&mut self.blend_messages).poll_next(&mut cx) {
            match msg {
                MixOutgoingMessage::Outbound(msg) => {
                    self.persistent_sender.send(msg).unwrap();
                }
                MixOutgoingMessage::FullyUnwrapped(_) => {
                    println!("fully unwrapped message: Node:{}", self.id);
                    self.state.num_messages_broadcasted += 1;
                    //TODO: create a tracing event
                }
            }
        }
        // Proceed persistent transmission
        if let Poll::Ready(Some(msg)) =
            pin!(&mut self.persistent_transmission_messages).poll_next(&mut cx)
        {
            self.forward(MixMessage(msg));
        }

        self.state.step_id += 1;
    }

    fn analyze(&self, ward: &mut WardCondition) -> bool {
        match ward {
            WardCondition::Max(_) => false,
            WardCondition::Sum(condition) => {
                *condition.step_result.borrow_mut() += self.state.num_messages_broadcasted;
                false
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct MessagePayload {
    node_id: [u8; 32],
    step_id: usize,
}
