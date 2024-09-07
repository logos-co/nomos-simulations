use glob::glob;
use polars::prelude::*;
use std::env;
use std::fs::File;
use walkdir::WalkDir;

fn aggregate(path: &str) {
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.path().file_name().unwrap().to_string_lossy();
        if dir_name.starts_with("paramset_") {
            let mut aggregated_series = Series::new_empty("", &DataType::Int64);
            let pattern = format!("{}/**/latency.csv", entry.path().display());

            for file in glob(&pattern).unwrap().filter_map(Result::ok) {
                let df = CsvReadOptions::default()
                    .with_has_header(true)
                    .try_into_reader_with_file_path(Some(file.clone()))
                    .unwrap()
                    .finish()
                    .unwrap();

                aggregated_series
                    .extend(
                        &df.column("latency")
                            .unwrap()
                            .i64()
                            .unwrap()
                            .clone()
                            .into_series(),
                    )
                    .unwrap();

                println!("Processed {}", file.display());
            }

            let output_file = format!("{}/latency_stats.csv", entry.path().display());
            save_stats(&aggregated_series, &output_file);
        }
    }
}

fn save_stats(aggregated: &Series, outpath: &str) {
    let min = aggregated.min::<i64>().unwrap();
    let max = aggregated.max::<i64>().unwrap();
    let mean = aggregated.mean().unwrap();
    let median = aggregated.median().unwrap();
    let std = aggregated.std(1).unwrap();

    let mut df = DataFrame::new(vec![
        Series::new("min", &[min]),
        Series::new("median", &[median]),
        Series::new("mean", &[mean]),
        Series::new("std", &[std]),
        Series::new("max", &[max]),
    ])
    .unwrap();

    let mut file = File::create(outpath).unwrap();
    CsvWriter::new(&mut file).finish(&mut df).unwrap();
    println!("Saved {}", outpath);
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
