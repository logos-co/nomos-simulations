# Ordering Experiments: Analysis Tools

This Python project contains scripts to draw plots and tables from the results of the [ordering experiments](../ordering).

## Prerequisites
- Python 3
- Installing dependencies
```bash
python3 -m venv .venv
source .venv/bin/activate
pip isntall -r requirements.txt
```

## Usages

### Analyzing the latency results

The following command draws plots and tables from the latency results of the ordering experiments.
```bash
python latency.py <aggregated_csv_path> <output_dir>
```
- `aggregated_csv_path`
  - A path to a CSV file that contains all statistical results of all experiments that must be shown in the plot and table.
  - This script expects that the CSV file has at least the following columns.
    - `paramset` (int): Parameter Set ID
    - `queue_type` (str): Queue Type (NonMix, PureCoinFlipping, ...)
    - `latency_min`, `latency_mean` and `latency_max` (float)
- `output_dir` A directory path where all PNG files of plots and tables are stored in


### Analyzing the ordering coefficient results

The following command draws plots and tables from the ordering coefficient results of the ordering experiments.
```bash
python coeff.py <aggregated_csv_path> <output_dir>
```
- `aggregated_csv_path`
  - A path to a CSV file that contains all statistical results of all experiments that must be shown in the plot and table.
  - This script expects that the CSV file has at least the following columns.
    - `paramset` (int): Parameter Set ID
    - `queue_type` (str): Queue Type (NonMix, PureCoinFlipping, ...)
    - `strong_coeff_min`, `strong_coeff_mean` and `strong_coeff_max` (float)
    - `casual_coeff_min`, `casual_coeff_mean` and `casual_coeff_max` (float)
    - `weak_coeff_min`, `weak_coeff_mean` and `weak_coeff_max` (float)
- `output_dir` A directory path where all PNG files of plots and tables are stored in
