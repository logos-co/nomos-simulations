# Nomos Node Simulations

This repository contains a suite of scripts and configurations for running and analyzing simulations of nomos nodes.

## Prerequisites

### 1. Nomos Node Simulation App
Before running simulations, ensure the `nomos-node/simulations` application is accessible from the `$PATH`:
- Clone the project:
```bash
git clone git@github.com:logos-co/nomos-node.git
```
- Build the project using Crago:
```bash
crago build -p simulations --release
```
- Add the resulting release directory to `$PATH`:
```bash
export PATH=$PATH:<path to nomos-node>/target/release
```

### 2. Python Dependencies
The data conversion and normalization processes make use of the Pandas Python package:
```bash
pip install pandas
```

## Network Overlays

The simulation application supports two distinct network topologies to link nodes:

- **Tree Overlay**: Constructs a full binary tree overlay, mirroring the connections between actual Nomos nodes.
- **Branch Overlay**: Produces a single full-length branch of the binary tree overlay. This overlay offers a simulation of a more extensive network (though with fewer nodes) and is intended to approximate the latencies of a complete binary tree. More details can be found on [Notion](https://www.notion.so/Carnot-Simulation-Mechanism-c025dbab6b374c139004aae45831cf78).

## Test Cases

The primary objective of the simulation app is to replicate a large-scale real-world network environment while running the Carnot consensus engine.

### Committee Sizes
The simulation configuration can specify varying numbers of committees and nodes within them. To obtain these values, as recommended in the Carnot spec, refer to the `committee_sizes.py` script available at [nomos-specs](https://github.com/logos-co/nomos-specs/blob/master/carnot/committee_sizes.py)

### Sample Test Cases
This repository includes a set of sample test cases under `scripts/test_cases.csv`. The `committee_sizes.py` script from nomos-specs was used to generate these test cases.

## Configuration

The simulation application operates with JSON configuration files. The `scripts` directory offers helper scripts for creating a range of config variations:

### `build_config.py`
Generates a single configuration based on the template JSON file located at `scripts/config_builder/template.json`.

Usage:
```bash
python build_config.py <tree/branch> <number of committees> <total nodes> <config name> <optional max_view to simulate> <optional network variation defined in config_builder/network>
```

### `build_cases.py`
Produces multiple config variations as defined in the provided test cases CSV file (see `test_cases.csv` for a reference).

Usage:
```bash
python build_cases.py test_cases.csv
```

## Running the Simulation

### Standalone Mode
Assuming the `simulations` binary is in your `$PATH`, run the simulation with your chosen config file:
```bash
simulations --input-settings <config_file.json> --stream type naive
```

### Batch Mode with Multiple Configurations
To execute a series of simulations using different configuration files, utilize the `run_configs.py` script:
```bash
python run_configs.py <configs_dir>
```
