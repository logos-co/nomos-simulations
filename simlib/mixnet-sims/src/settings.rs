use netrunner::settings::SimulationSettings;
use nomos_mix::persistent_transmission::PersistentTransmissionSettings;
use serde::{Deserialize, Deserializer};
use std::time::Duration;

#[derive(Deserialize)]
pub struct SimSettings {
    #[serde(flatten)]
    pub simulation_settings: SimulationSettings,
    pub connected_peers_count: usize,
    #[serde(deserialize_with = "deserialize_duration_with_human_time")]
    pub data_message_lottery_interval: Duration,
    pub stake_proportion: f64,
    #[serde(deserialize_with = "deserialize_duration_with_human_time")]
    pub epoch_duration: Duration,
    #[serde(deserialize_with = "deserialize_duration_with_human_time")]
    pub slot_duration: Duration,
    pub persistent_transmission: PersistentTransmissionSettings,
    pub number_of_mix_layers: usize,
    pub max_delay_seconds: u64,
    pub slots_per_epoch: usize,
}

fn deserialize_duration_with_human_time<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    humantime::parse_duration(&s).map_err(serde::de::Error::custom)
}
