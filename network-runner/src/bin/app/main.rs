// std
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// crates
use anyhow::Ok;
use clap::Parser;
use crossbeam::channel;
use nomos_simulations_network_runner::network::behaviour::create_behaviours;
use nomos_simulations_network_runner::network::regions::{create_regions, RegionsData};
use nomos_simulations_network_runner::network::{InMemoryNetworkInterface, Network};
use nomos_simulations_network_runner::node::mix::state::{MixnodeRecord, MixnodeState};
use nomos_simulations_network_runner::node::mix::{MixMessage, MixNode, MixnodeSettings};
use nomos_simulations_network_runner::node::{NodeId, NodeIdExt};
use nomos_simulations_network_runner::output_processors::Record;
use nomos_simulations_network_runner::runner::{BoxedNode, SimulationRunnerHandle};
#[cfg(feature = "polars")]
use nomos_simulations_network_runner::streaming::polars::PolarsSubscriber;
use nomos_simulations_network_runner::streaming::{
    io::IOSubscriber, naive::NaiveSubscriber, StreamType,
};
use parking_lot::Mutex;
use rand::prelude::IteratorRandom;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::de::DeserializeOwned;
use serde::Serialize;
// internal
use nomos_simulations_network_runner::{runner::SimulationRunner, settings::SimulationSettings};
mod log;

/// Main simulation wrapper
/// Pipes together the cli arguments with the execution
#[derive(Parser)]
pub struct SimulationApp {
    /// Json file path, on `SimulationSettings` format
    #[clap(long, short)]
    input_settings: PathBuf,
    #[clap(long)]
    stream_type: Option<StreamType>,
    #[clap(long, default_value = "plain")]
    log_format: log::LogFormat,
    #[clap(long, default_value = "stdout")]
    log_to: log::LogOutput,
    #[clap(long)]
    no_netcap: bool,
}

impl SimulationApp {
    pub fn run(self) -> anyhow::Result<()> {
        let Self {
            input_settings,
            stream_type,
            log_format: _,
            log_to: _,
            no_netcap,
        } = self;
        let simulation_settings: SimulationSettings = load_json_from_file(&input_settings)?;

        let seed = simulation_settings.seed.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
        });
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut node_ids: Vec<NodeId> = (0..simulation_settings.node_count)
            .map(NodeId::from_index)
            .collect();
        node_ids.shuffle(&mut rng);

        let regions = create_regions(&node_ids, &mut rng, &simulation_settings.network_settings);
        let behaviours = create_behaviours(&simulation_settings.network_settings);
        let regions_data = RegionsData::new(regions, behaviours);

        let ids = node_ids.clone();
        let network = Arc::new(Mutex::new(Network::<MixMessage>::new(regions_data, seed)));

        let nodes: Vec<_> = node_ids
            .iter()
            .copied()
            .map(|node_id| {
                let mut network = network.lock();
                create_boxed_mixnode(
                    node_id,
                    &mut network,
                    simulation_settings.clone(),
                    no_netcap,
                    MixnodeSettings {
                        connected_peers: ids
                            .iter()
                            .filter(|&id| id != &node_id)
                            .copied()
                            .choose_multiple(&mut rng, 3),
                    },
                )
            })
            .collect();
        let network = Arc::try_unwrap(network)
            .expect("network is not used anywhere else")
            .into_inner();
        run::<_, _, _>(network, nodes, simulation_settings, stream_type)?;
        Ok(())
    }
}

fn create_boxed_mixnode(
    node_id: NodeId,
    network: &mut Network<MixMessage>,
    simulation_settings: SimulationSettings,
    no_netcap: bool,
    mixnode_settings: MixnodeSettings,
) -> BoxedNode<MixnodeSettings, MixnodeState> {
    let (node_message_broadcast_sender, node_message_broadcast_receiver) = channel::unbounded();
    let (node_message_sender, node_message_receiver) = channel::unbounded();
    // Dividing milliseconds in second by milliseconds in the step.
    let step_time_as_second_fraction =
        simulation_settings.step_time.subsec_millis() as f32 / 1_000_000_f32;
    let capacity_bps = if no_netcap {
        None
    } else {
        simulation_settings
            .node_settings
            .network_capacity_kbps
            .map(|c| (c as f32 * 1024.0 * step_time_as_second_fraction) as u32)
    };
    let network_message_receiver = {
        network.connect(
            node_id,
            capacity_bps,
            node_message_receiver,
            node_message_broadcast_receiver,
        )
    };
    let network_interface = InMemoryNetworkInterface::new(
        node_id,
        node_message_broadcast_sender,
        node_message_sender,
        network_message_receiver,
    );
    Box::new(MixNode::new(node_id, mixnode_settings, network_interface))
}

fn run<M, S, T>(
    network: Network<M>,
    nodes: Vec<BoxedNode<S, T>>,
    settings: SimulationSettings,
    stream_type: Option<StreamType>,
) -> anyhow::Result<()>
where
    M: std::fmt::Debug + Clone + Send + Sync + 'static,
    S: 'static,
    T: Serialize + Clone + 'static,
{
    let stream_settings = settings.stream_settings.clone();
    let runner = SimulationRunner::<_, MixnodeRecord, S, T>::new(
        network,
        nodes,
        Default::default(),
        settings,
    )?;

    let handle = match stream_type {
        Some(StreamType::Naive) => {
            let settings = stream_settings.unwrap_naive();
            runner.simulate_and_subscribe::<NaiveSubscriber<MixnodeRecord>>(settings)?
        }
        Some(StreamType::IO) => {
            let settings = stream_settings.unwrap_io();
            runner.simulate_and_subscribe::<IOSubscriber<MixnodeRecord>>(settings)?
        }
        #[cfg(feature = "polars")]
        Some(StreamType::Polars) => {
            let settings = stream_settings.unwrap_polars();
            runner.simulate_and_subscribe::<PolarsSubscriber<MixnodeRecord>>(settings)?
        }
        None => runner.simulate()?,
    };

    signal(handle)
}

fn signal<R: Record>(handle: SimulationRunnerHandle<R>) -> anyhow::Result<()> {
    let handle = Arc::new(handle);
    let (tx, rx) = crossbeam::channel::bounded(1);
    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })?;
    loop {
        crossbeam::select! {
            recv(rx) -> _ => {
                handle.stop()?;
                tracing::info!("gracefully shutdown the simulation app");
                break;
            },
            default => {
                if handle.is_finished() {
                    handle.shutdown()?;
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}

/// Generically load a json file
fn load_json_from_file<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let f = File::open(path).map_err(Box::new)?;
    Ok(serde_json::from_reader(f)?)
}

fn main() -> anyhow::Result<()> {
    let app: SimulationApp = SimulationApp::parse();
    log::config_tracing(app.log_format, &app.log_to);

    if let Err(e) = app.run() {
        tracing::error!("error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}
