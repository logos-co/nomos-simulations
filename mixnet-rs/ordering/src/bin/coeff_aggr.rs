use glob::glob;
use polars::prelude::*;
use std::env;
use std::fs::File;
use walkdir::WalkDir;

fn aggregate(path: &str) {
    let mut input_schema = Schema::new();
    input_schema.with_column("sender".into(), DataType::UInt64);
    input_schema.with_column("receiver".into(), DataType::UInt64);
    input_schema.with_column("strong".into(), DataType::UInt64);
    input_schema.with_column("casual".into(), DataType::UInt64);
    input_schema.with_column("weak".into(), DataType::UInt64);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.path().file_name().unwrap().to_string_lossy();
        if dir_name.starts_with("paramset_") {
            let mut strongs = Series::new_empty("", &DataType::UInt64);
            let mut casuals = Series::new_empty("", &DataType::UInt64);
            let mut weaks = Series::new_empty("", &DataType::UInt64);

            let pattern = format!("{}/**/coeffs_*.csv", entry.path().display());
            for file in glob(&pattern).unwrap().filter_map(Result::ok) {
                let df = CsvReadOptions::default()
                    .with_has_header(true)
                    .with_schema(Some(SchemaRef::new(input_schema.clone())))
                    .try_into_reader_with_file_path(Some(file.clone()))
                    .unwrap()
                    .finish()
                    .unwrap();

                extend_series(&mut strongs, &df, "strong");
                extend_series(&mut casuals, &df, "casual");
                extend_series(&mut weaks, &df, "weak");

                println!("Processed {}", file.display());
            }

            save_stats(
                &strongs,
                &format!("{}/strong_coeff_stats.csv", entry.path().display()),
            );
            save_stats(
                &casuals,
                &format!("{}/casual_coeff_stats.csv", entry.path().display()),
            );
            save_stats(
                &weaks,
                &format!("{}/weak_coeff_stats.csv", entry.path().display()),
            );
        }
    }
}

fn extend_series(series: &mut Series, df: &DataFrame, column: &str) {
    series
        .extend(
            &df.column(column)
                .unwrap()
                .u64()
                .unwrap()
                .clone()
                .into_series(),
        )
        .unwrap();
}

fn save_stats(aggregated: &Series, outpath: &str) {
    let mut df = DataFrame::new(vec![
        Series::new("min", &[aggregated.min::<u64>().unwrap()]),
        Series::new("median", &[aggregated.median().unwrap()]),
        Series::new("mean", &[aggregated.mean().unwrap()]),
        Series::new("std", &[aggregated.std(1).unwrap()]),
        Series::new("max", &[aggregated.max::<u64>().unwrap()]),
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
