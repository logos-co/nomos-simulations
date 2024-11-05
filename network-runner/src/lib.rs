pub mod network;
pub mod node;
pub mod output_processors;
pub mod runner;
pub mod settings;
pub mod streaming;
pub mod warding;

static START_TIME: once_cell::sync::Lazy<std::time::Instant> =
    once_cell::sync::Lazy::new(std::time::Instant::now);
