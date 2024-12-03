# Nomos Blend Simulation Result Analysis

First of all, run the Nomos Blend simulation by following the [instruction](../simlib/mixnet-sims/).

## Latency Analysis

```bash
python3 latency.py <log_path>
```
This script calculates the minimum, average, median, and maximum latency taken by data messages to reach the final blend node.

## Anonymity Analysis

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

python3 emission.py --log-path <log_path> --plot-png-path <plot_png_path>
```
This script draws a scatter plot of emission events from all nodes
to see how many messages (data or cover) were emitted at the same time window.
