use netrunner::settings::SimulationSettings;
use nomos_mix::persistent_transmission::PersistentTransmissionSettings;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
pub struct SimSettings {
    #[serde(flatten)]
    pub simulation_settings: SimulationSettings,
    pub connected_peers_count: usize,
    pub data_message_lottery_interval: Duration,
    pub stake_proportion: f64,
    pub seed: u64,
    pub epoch_duration: Duration,
    pub slot_duration: Duration,
    pub persistent_transmission: PersistentTransmissionSettings,
    pub number_of_mix_layers: usize,
    pub max_delay_secconds: u64,
    pub slots_per_epoch: usize,
}
