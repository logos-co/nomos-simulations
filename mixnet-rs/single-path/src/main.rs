mod paramset;

use clap::Parser;
use paramset::{ExperimentId, SessionId};
use queue::QueueType;

#[derive(Debug, Parser)]
#[command(name = "Single Sender Single Mix Measurement")]
struct Args {
    #[arg(short, long)]
    exp_id: ExperimentId,
    #[arg(short, long)]
    session_id: SessionId,
    #[arg(short, long)]
    queue_type: QueueType,
    #[arg(short, long)]
    outdir: String,
    #[arg(short, long)]
    from_paramset: Option<u16>,
}

fn main() {
    println!("Hello, world!");
}
