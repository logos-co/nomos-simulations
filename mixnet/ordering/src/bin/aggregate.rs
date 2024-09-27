use std::{
    env,
    fs::File,
    path::{Path, PathBuf},
};

use polars::prelude::*;
use walkdir::WalkDir;

fn aggregate(path: &str) {
    let mut schema = Schema::new();
    schema.with_column("paramset".into(), DataType::Int64);
    schema.with_column("num_mixes".into(), DataType::Int64);
    schema.with_column("num_paths".into(), DataType::Int64);
    schema.with_column("random_topology".into(), DataType::Boolean);
    schema.with_column("peering_degree".into(), DataType::String);
    schema.with_column("min_queue_size".into(), DataType::Int64);
    schema.with_column("transmission_rate".into(), DataType::Float32);
    schema.with_column("num_senders".into(), DataType::Int64);
    schema.with_column("num_sender_msgs".into(), DataType::Int64);
    schema.with_column("num_sender_data_msgs".into(), DataType::Int64);
    schema.with_column("sender_data_msg_prob".into(), DataType::Float32);
    schema.with_column("sender_data_msg_interval".into(), DataType::Float32);
    schema.with_column("mix_data_msg_prob".into(), DataType::Float32);
    schema.with_column("num_mixes_sending_data".into(), DataType::Int64);
    schema.with_column("mix_data_msg_interval".into(), DataType::Float32);
    schema.with_column("queue_type".into(), DataType::String);
    schema.with_column("num_iterations".into(), DataType::Int64);

    let mut dataframes: Vec<DataFrame> = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.path().file_name().unwrap().to_string_lossy();
        if dir_name.starts_with("paramset_") {
            let mut df = CsvReadOptions::default()
                .with_has_header(true)
                .with_schema(Some(SchemaRef::new(schema.clone())))
                .try_into_reader_with_file_path(Some(entry.path().join("paramset.csv")))
                .unwrap()
                .finish()
                .unwrap();

            // add_stats_columns(
            //     &mut df,
            //     entry.path().join("data_msg_counts_stats.csv"),
            //     "data_msg_count_",
            // );
            add_stats_columns(&mut df, entry.path().join("latency_stats.csv"), "latency_");
            add_stats_columns(&mut df, entry.path().join("hops_stats.csv"), "hops_");

            for prefix in ["strong", "casual", "weak"] {
                let coeff_stats_path = entry.path().join(format!("{}_coeff_stats.csv", prefix));
                if coeff_stats_path.exists() {
                    add_stats_columns(&mut df, coeff_stats_path, &format!("{}_coeff_", prefix));
                }
            }

            dataframes.push(df);
        }
    }

    if !dataframes.is_empty() {
        let df = polars::functions::concat_df_diagonal(dataframes.as_slice()).unwrap();
        let mut df = df
            .sort(["paramset", "queue_type"], SortMultipleOptions::default())
            .unwrap();
        let outpath = Path::new(path).join("aggregated.csv");
        let mut file = File::create(&outpath).unwrap();
        CsvWriter::new(&mut file).finish(&mut df).unwrap();
        println!("Saved {}", outpath.display());
    }
}

fn add_stats_columns(df: &mut DataFrame, path: PathBuf, col_prefix: &str) {
    let mut schema = Schema::new();
    schema.with_column("min".into(), DataType::Float64);
    schema.with_column("median".into(), DataType::Float64);
    schema.with_column("mean".into(), DataType::Float64);
    schema.with_column("std".into(), DataType::Float64);
    schema.with_column("max".into(), DataType::Float64);

    let stats_df = CsvReadOptions::default()
        .with_has_header(true)
        .with_schema(Some(SchemaRef::new(schema)))
        .try_into_reader_with_file_path(Some(path))
        .unwrap()
        .finish()
        .unwrap();
    df.with_column(
        stats_df["min"]
            .head(Some(1))
            .with_name(format!("{col_prefix}min").as_str()),
    )
    .unwrap();
    df.with_column(
        stats_df["median"]
            .head(Some(1))
            .with_name(format!("{col_prefix}median").as_str()),
    )
    .unwrap();
    df.with_column(
        stats_df["mean"]
            .head(Some(1))
            .with_name(format!("{col_prefix}mean").as_str()),
    )
    .unwrap();
    df.with_column(
        stats_df["std"]
            .head(Some(1))
            .with_name(format!("{col_prefix}std").as_str()),
    )
    .unwrap();
    df.with_column(
        stats_df["max"]
            .head(Some(1))
            .with_name(format!("{col_prefix}max").as_str()),
    )
    .unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];
    aggregate(path);
}
