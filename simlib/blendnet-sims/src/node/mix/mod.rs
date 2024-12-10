pub mod consensus_streams;
pub mod lottery;
mod message;
pub mod scheduler;
pub mod state;
pub mod stream_wrapper;
pub mod topology;

use crate::node::mix::consensus_streams::{Epoch, Slot};
use cached::{Cached, TimedCache};
use crossbeam::channel;
use futures::Stream;
use lottery::StakeLottery;
use message::{Payload, PayloadId};
use netrunner::network::NetworkMessage;
use netrunner::node::{Node, NodeId, NodeIdExt};
use netrunner::{
    network::{InMemoryNetworkInterface, NetworkInterface, PayloadSize},
    warding::WardCondition,
};
use nomos_mix::conn_maintenance::{ConnectionMaintenance, ConnectionMaintenanceSettings};
use nomos_mix::{
    cover_traffic::{CoverTraffic, CoverTrafficSettings},
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
use polars::series::Series;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use scheduler::{Interval, TemporalRelease};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use state::MixnodeState;
use std::{pin::pin, task::Poll, time::Duration};
use stream_wrapper::CrossbeamReceiverStream;
use topology::Topology;

#[derive(Debug, Clone)]
pub struct MixMessage(Vec<u8>);

impl PayloadSize for MixMessage {
    fn size_bytes(&self) -> u32 {
        2208
    }
}

#[derive(Deserialize)]
pub struct MixnodeSettings {
    pub membership: Vec<NodeId>,
    pub topology: Topology,
    pub data_message_lottery_interval: Duration,
    pub stake_proportion: f64,
    pub seed: u64,
    pub epoch_duration: Duration,
    pub slot_duration: Duration,
    pub conn_maintenance: ConnectionMaintenanceSettings,
    pub persistent_transmission: PersistentTransmissionSettings,
    pub message_blend: MessageBlendSettings<MockMixMessage>,
    pub cover_traffic_settings: CoverTrafficSettings,
}

type Sha256Hash = [u8; 32];

/// This node implementation only used for testing different streaming implementation purposes.
pub struct MixNode {
    id: NodeId,
    state: MixnodeState,
    network_interface: InMemoryNetworkInterface<MixMessage>,
    message_cache: TimedCache<Sha256Hash, ()>,

    data_msg_lottery_update_time_sender: channel::Sender<Duration>,
    data_msg_lottery_interval: Interval,
    data_msg_lottery: StakeLottery<ChaCha12Rng>,

    conn_maintenance: ConnectionMaintenance<NodeId, MockMixMessage, ChaCha12Rng>,
    conn_maintenance_update_time_sender: channel::Sender<Duration>,
    conn_maintenance_interval: Interval,
    persistent_sender: channel::Sender<Vec<u8>>,
    persistent_update_time_sender: channel::Sender<Duration>,
    persistent_transmission_messages: PersistentTransmissionStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        MockMixMessage,
        Interval,
    >,
    crypto_processor: CryptographicProcessor<ChaCha12Rng, NodeId, MockMixMessage>,
    blend_sender: channel::Sender<Vec<u8>>,
    blend_update_time_sender: channel::Sender<Duration>,
    blend_messages: MessageBlendStream<
        CrossbeamReceiverStream<Vec<u8>>,
        ChaCha12Rng,
        NodeId,
        MockMixMessage,
        TemporalRelease,
    >,
    epoch_update_sender: channel::Sender<Duration>,
    slot_update_sender: channel::Sender<Duration>,
    cover_traffic: CoverTraffic<Epoch, Slot, MockMixMessage>,
}

impl MixNode {
    pub fn new(
        id: NodeId,
        settings: MixnodeSettings,
        network_interface: InMemoryNetworkInterface<MixMessage>,
    ) -> Self {
        let mut rng_generator = ChaCha12Rng::seed_from_u64(settings.seed);

        // Init Membership
        let nodes: Vec<
            nomos_mix::membership::Node<
                NodeId,
                <MockMixMessage as nomos_mix_message::MixMessage>::PublicKey,
            >,
        > = settings
            .membership
            .iter()
            .map(|&node_id| nomos_mix::membership::Node {
                address: node_id,
                public_key: node_id.into(),
            })
            .collect();
        let membership = Membership::<NodeId, MockMixMessage>::new(nodes, id.into());

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

        // Init Tier-1: Connection maintenance and Persistent transmission
        let mut conn_maintenance = ConnectionMaintenance::new(
            settings.conn_maintenance,
            membership.clone(),
            ChaCha12Rng::from_rng(&mut rng_generator).unwrap(),
        );
        settings
            .topology
            .get(&id)
            .unwrap()
            .iter()
            .for_each(|peer| conn_maintenance.add_connected_peer(*peer));
        let (conn_maintenance_update_time_sender, conn_maintenance_update_time_receiver) =
            channel::unbounded();
        let (persistent_sender, persistent_receiver) = channel::unbounded();
        let conn_maintenance_interval = Interval::new(
            settings.conn_maintenance.monitor.unwrap().time_window,
            conn_maintenance_update_time_receiver,
        );
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

        // tier 3 cover traffic
        let (epoch_update_sender, epoch_updater_update_receiver) = channel::unbounded();
        let (slot_update_sender, slot_updater_update_receiver) = channel::unbounded();
        let cover_traffic: CoverTraffic<Epoch, Slot, MockMixMessage> = CoverTraffic::new(
            settings.cover_traffic_settings,
            Epoch::new(settings.epoch_duration, epoch_updater_update_receiver),
            Slot::new(
                settings.cover_traffic_settings.slots_per_epoch,
                settings.slot_duration,
                slot_updater_update_receiver,
            ),
        );

        Self {
            id,
            network_interface,
            // We're not coupling this lifespan with the steps now, but it's okay
            // We expected that a message will be delivered to most of nodes within 60s.
            message_cache: TimedCache::with_lifespan(60),
            state: MixnodeState {
                node_id: id,
                step_id: 0,
                num_messages_fully_unwrapped: 0,
            },
            data_msg_lottery_update_time_sender,
            data_msg_lottery_interval,
            data_msg_lottery,
            conn_maintenance,
            conn_maintenance_update_time_sender,
            conn_maintenance_interval,
            persistent_sender,
            persistent_update_time_sender,
            persistent_transmission_messages,
            crypto_processor,
            blend_sender,
            blend_update_time_sender,
            blend_messages,
            epoch_update_sender,
            slot_update_sender,
            cover_traffic,
        }
    }

    fn forward(
        &mut self,
        message: MixMessage,
        exclude_node: Option<NodeId>,
        log: Option<EmissionLog>,
    ) {
        for (i, node_id) in self
            .conn_maintenance
            .connected_peers()
            .iter()
            .filter(|&id| Some(*id) != exclude_node)
            .enumerate()
        {
            if i == 0 {
                if let Some(log) = &log {
                    Self::log_emission(log);
                }
            }
            self.network_interface
                .send_message(*node_id, message.clone())
        }
        self.message_cache.cache_set(Self::sha256(&message.0), ());
    }

    fn receive(&mut self) -> Vec<NetworkMessage<MixMessage>> {
        self.network_interface
            .receive_messages()
            .into_iter()
            .inspect(|msg| {
                self.conn_maintenance.record_effective_message(&msg.from);
            })
            // Retain only messages that have not been seen before
            .filter(|msg| {
                self.message_cache
                    .cache_set(Self::sha256(&msg.payload().0), ())
                    .is_none()
            })
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
        self.conn_maintenance_update_time_sender
            .send(elapsed)
            .unwrap();
        self.persistent_update_time_sender.send(elapsed).unwrap();
        self.blend_update_time_sender.send(elapsed).unwrap();
        self.epoch_update_sender.send(elapsed).unwrap();
        self.slot_update_sender.send(elapsed).unwrap();
    }

    fn log_message_generated(&self, msg_type: &str, payload: &Payload) {
        self.log_message(format!("{}MessageGenerated", msg_type).as_str(), payload);
    }

    fn log_message_fully_unwrapped(&self, payload: &Payload) {
        self.log_message("MessageFullyUnwrapped", payload);
    }

    fn log_message(&self, tag: &str, payload: &Payload) {
        let log = MessageLog {
            payload_id: payload.id(),
            step_id: self.state.step_id,
            node_id: self.id.index(),
        };
        tracing::info!("{}: {}", tag, serde_json::to_string(&log).unwrap());
    }

    fn log_emission(log: &EmissionLog) {
        tracing::info!("Emission: {}", serde_json::to_string(log).unwrap());
    }

    fn log_monitors(&self, effective_messages_series: &Series) {
        if effective_messages_series.is_empty() {
            return;
        }

        let log = MonitorsLog {
            node_id: self.id.index(),
            message_type: "EffectiveMessage".to_string(),
            num_conns: effective_messages_series.len(),
            min: effective_messages_series.min().unwrap().unwrap(),
            avg: effective_messages_series.mean().unwrap(),
            median: effective_messages_series.median().unwrap(),
            max: effective_messages_series.max().unwrap().unwrap(),
        };
        tracing::info!("Monitor: {}", serde_json::to_string(&log).unwrap());
    }

    fn new_emission_log(&self, emission_type: &str) -> EmissionLog {
        EmissionLog {
            emission_type: emission_type.to_string(),
            step_id: self.state.step_id,
            node_id: self.id.index(),
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
        self.update_time(elapsed);
        let waker = futures::task::noop_waker();
        let mut cx = futures::task::Context::from_waker(&waker);

        // Generate a data message probabilistically
        if let Poll::Ready(Some(_)) = pin!(&mut self.data_msg_lottery_interval).poll_next(&mut cx) {
            if self.data_msg_lottery.run() {
                let payload = Payload::new();
                self.log_message_generated("Data", &payload);
                let message = self
                    .crypto_processor
                    .wrap_message(payload.as_bytes())
                    .unwrap();
                self.persistent_sender.send(message).unwrap();
            }
        }

        // Proceed connection maintenance if interval is reached.
        if let Poll::Ready(Some(_)) = pin!(&mut self.conn_maintenance_interval).poll_next(&mut cx) {
            let (monitors, _, _) = self.conn_maintenance.reset().unwrap();
            let effective_messages_series = Series::from_iter(
                monitors
                    .values()
                    .map(|monitor| monitor.effective_messages.to_num::<u64>()),
            );
            self.log_monitors(&effective_messages_series);
        }

        // Handle incoming messages
        for network_message in self.receive() {
            self.forward(
                network_message.payload().clone(),
                Some(network_message.from),
                None,
            );
            self.blend_sender
                .send(network_message.into_payload().0)
                .unwrap();
        }

        // Proceed message blend
        if let Poll::Ready(Some(msg)) = pin!(&mut self.blend_messages).poll_next(&mut cx) {
            match msg {
                MixOutgoingMessage::Outbound(msg) => {
                    self.persistent_sender.send(msg).unwrap();
                }
                MixOutgoingMessage::FullyUnwrapped(payload) => {
                    let payload = Payload::load(payload);
                    self.log_message_fully_unwrapped(&payload);
                    self.state.num_messages_fully_unwrapped += 1;
                    //TODO: create a tracing event
                }
            }
        }

        // Generate a cover message probabilistically
        if let Poll::Ready(Some(_)) = pin!(&mut self.cover_traffic).poll_next(&mut cx) {
            let payload = Payload::new();
            self.log_message_generated("Cover", &payload);
            let message = self
                .crypto_processor
                .wrap_message(payload.as_bytes())
                .unwrap();
            self.persistent_sender.send(message).unwrap();
        }

        // Proceed persistent transmission
        if let Poll::Ready(Some(msg)) =
            pin!(&mut self.persistent_transmission_messages).poll_next(&mut cx)
        {
            self.forward(
                MixMessage(msg),
                None,
                Some(self.new_emission_log("FromPersistent")),
            );
        }

        self.state.step_id += 1;
    }

    fn analyze(&self, ward: &mut WardCondition) -> bool {
        match ward {
            WardCondition::Max(_) => false,
            WardCondition::Sum(condition) => {
                *condition.step_result.borrow_mut() += self.state.num_messages_fully_unwrapped;
                false
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageLog {
    payload_id: PayloadId,
    step_id: usize,
    node_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmissionLog {
    emission_type: String,
    step_id: usize,
    node_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct MonitorsLog {
    node_id: usize,
    message_type: String,
    num_conns: usize,
    min: u64,
    avg: f64,
    median: f64,
    max: u64,
}
