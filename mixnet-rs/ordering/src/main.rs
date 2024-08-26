mod iteration;
mod message;
mod ordercoeff;
mod outputs;
mod paramset;
mod topology;

use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    path::Path,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use clap::Parser;
use iteration::Iteration;
use paramset::{ExperimentId, ParamSet, SessionId, PARAMSET_CSV_COLUMNS};
use protocol::queue::QueueType;

#[derive(Debug, Parser)]
#[command(name = "Ordering Measurement")]
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
    num_threads: usize,
    #[arg(short, long, default_value_t = false)]
    reverse_order: bool,
    #[arg(short, long)]
    from_paramset: Option<u16>,
    #[arg(short, long)]
    to_paramset: Option<u16>,
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
        num_threads,
        reverse_order,
        from_paramset,
        to_paramset,
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
    let rootdir = format!("{outdir}/{subdir}");
    std::fs::create_dir_all(&rootdir).unwrap();

    let paramsets = ParamSet::new_all_paramsets(exp_id, session_id, queue_type);

    let session_start_time = SystemTime::now();

    let iterations = prepare_all_iterations(
        &paramsets,
        from_paramset,
        to_paramset,
        reverse_order,
        &rootdir,
    );
    run_all_iterations(iterations, num_threads, paramsets.len());

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

fn prepare_all_iterations(
    paramsets: &[ParamSet],
    from_paramset: Option<u16>,
    to_paramset: Option<u16>,
    reverse_order: bool,
    rootdir: &str,
) -> Vec<Iteration> {
    let mut iterations: Vec<Iteration> = Vec::new();
    for paramset in paramsets.iter() {
        if paramset.id < from_paramset.unwrap_or(0) {
            tracing::info!("ParamSet:{} skipped", paramset.id);
            continue;
        } else if paramset.id > to_paramset.unwrap_or(u16::MAX) {
            tracing::info!("ParamSets:{}~ skipped", paramset.id);
            break;
        }

        let paramset_dir = format!("{rootdir}/__WIP__paramset_{}", paramset.id);
        std::fs::create_dir_all(paramset_dir.as_str()).unwrap();
        save_paramset_info(paramset, format!("{paramset_dir}/paramset.csv").as_str()).unwrap();

        for i in 0..paramset.num_iterations {
            iterations.push(Iteration {
                paramset: paramset.clone(),
                iteration_idx: i,
                paramset_dir: paramset_dir.clone(),
            });
        }
    }

    if reverse_order {
        iterations.reverse();
    }
    iterations
}

fn run_all_iterations(iterations: Vec<Iteration>, num_threads: usize, num_paramsets: usize) {
    let (task_tx, task_rx) = crossbeam::channel::unbounded::<Iteration>();
    let (noti_tx, noti_rx) = crossbeam::channel::unbounded::<Iteration>();

    let mut threads = Vec::with_capacity(num_threads);
    for _ in 0..num_threads {
        let task_rx = task_rx.clone();
        let noti_tx = noti_tx.clone();

        let thread = std::thread::spawn(move || {
            while let Ok(mut iteration) = task_rx.recv() {
                iteration.start();
                noti_tx.send(iteration).unwrap();
            }
        });
        threads.push(thread);
    }

    let num_all_iterations = iterations.len();
    for iteration in iterations {
        task_tx.send(iteration).unwrap();
    }
    // Close the task sender channel, so that the threads can know that there's no task remains.
    drop(task_tx);

    let mut paramset_progresses: HashMap<u16, usize> = HashMap::new();
    let mut num_done_paramsets = 0;
    for _ in 0..num_all_iterations {
        let iteration = noti_rx.recv().unwrap();

        match paramset_progresses.entry(iteration.paramset.id) {
            Entry::Occupied(mut e) => {
                *e.get_mut() += 1;
            }
            Entry::Vacant(e) => {
                e.insert(1);
            }
        }

        if *paramset_progresses.get(&iteration.paramset.id).unwrap()
            == iteration.paramset.num_iterations
        {
            num_done_paramsets += 1;
            let new_paramset_dir = iteration
                .paramset_dir
                .replace("__WIP__paramset", "paramset");
            std::fs::rename(iteration.paramset_dir, new_paramset_dir).unwrap();
            tracing::info!(
                "ParamSet:{} is done ({} iterations). {}/{} ParamSets done.",
                iteration.paramset.id,
                iteration.paramset.num_iterations,
                num_done_paramsets,
                num_paramsets,
            );
        }
    }

    for thread in threads {
        thread.join().unwrap();
    }
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
