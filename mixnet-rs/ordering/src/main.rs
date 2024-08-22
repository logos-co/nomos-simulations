mod iteration;
mod ordercoeff;
mod paramset;

use std::{
    error::Error,
    path::Path,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use clap::Parser;
use iteration::run_iteration;
use paramset::{ExperimentId, ParamSet, SessionId, PARAMSET_CSV_COLUMNS};
use protocol::queue::QueueType;

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
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    tracing::info!("Arguments: {:?}", args);
    let Args {
        exp_id,
        session_id,
        queue_type,
        outdir,
        from_paramset,
    } = args;

    // Create a directory and initialize a CSV file only with a header
    assert!(
        Path::new(&outdir).is_dir(),
        "Output directory does not exist: {outdir}"
    );
    let subdir = format!(
        "__WIP__ordering_e{}s{}_{:?}_{}___DUR__",
        exp_id as u8,
        session_id as u8,
        queue_type,
        Utc::now().to_rfc3339()
    );
    std::fs::create_dir_all(&format!("{outdir}/{subdir}")).unwrap();

    let paramsets = ParamSet::new_all_paramsets(exp_id, session_id, queue_type);

    let session_start_time = SystemTime::now();

    for paramset in paramsets {
        if paramset.id < from_paramset.unwrap_or(0) {
            tracing::info!("ParamSet:{} skipped", paramset.id);
            continue;
        }

        let paramset_dir = format!("{outdir}/{subdir}/__WIP__paramset_{}", paramset.id);
        std::fs::create_dir_all(paramset_dir.as_str()).unwrap();
        save_paramset_info(&paramset, format!("{paramset_dir}/paramset.csv").as_str()).unwrap();

        for i in 0..paramset.num_iterations {
            run_iteration(
                paramset.clone(),
                i as u64,
                &format!("{paramset_dir}/iteration_{i}_latency.csv"),
                &format!("{paramset_dir}/iteration_{i}_sent_seq.csv"),
                &format!("{paramset_dir}/iteration_{i}_recv_seq.csv"),
                &format!("{paramset_dir}/iteration_{i}_data_msg_counts.csv"),
                &format!("{paramset_dir}/iteration_{i}_ordering_coeff.csv"),
            );
            tracing::info!("ParamSet:{}, Iteration:{} completed.", paramset.id, i);
        }
        let new_paramset_dir = paramset_dir.replace("__WIP__paramset_", "paramset_");
        std::fs::rename(&paramset_dir, &new_paramset_dir)
            .expect("Failed to rename: {paramset_dir} -> {new_paramset_dir}: {e}");

        tracing::info!("ParamSet:{} completed", paramset.id);
    }

    let session_duration = SystemTime::now()
        .duration_since(session_start_time)
        .unwrap();

    // Replace "__WIP__" and "__DUR__" in the subdir string
    let new_subdir = subdir
        .replace("__WIP__", "")
        .replace("__DUR__", &format_duration(session_duration));
    let old_path = format!("{}/{}", outdir, subdir);
    let new_path = format!("{}/{}", outdir, new_subdir);
    assert!(
        !Path::new(&new_path).exists(),
        "The new directory already exists: {new_path}"
    );
    std::fs::rename(&old_path, &new_path)
        .expect("Failed to rename the directory: {old_path} -> {new_path}");

    tracing::info!("Session completed.");
}

fn save_paramset_info(paramset: &ParamSet, path: &str) -> Result<(), Box<dyn Error>> {
    // Assert that the file does not already exist
    assert!(
        !Path::new(path).exists(),
        "File already exists at path: {path}",
    );

    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(PARAMSET_CSV_COLUMNS)?;
    wtr.write_record(paramset.as_csv_record())?;
    wtr.flush()?;

    Ok(())
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();

    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    format!("{}d{}h{}m{}s", days, hours, minutes, seconds)
}
