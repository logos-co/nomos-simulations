# Queuing Mechanism: Ordering Experiments

This directory contains the code for the [Ordering Experiments](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#4d38e8790ecd492a812c733bf140b864), which is the part of the Queuing Mechanism Experiments.

## Usages

```bash
cargo install --path ordering

ordering --exp-id 1 --session-id 1 --queue-type PureCoinFlipping --outdir $PWD/out --num-threads 4
```
- `exp-id`: [Experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#ffbcb5071dbb482bad035ef01cf8d49d) ID (starting from 1)
- `session-id`: [Session](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#df7de2a64b1e4e778c5d793bf03be25e) ID (starting from 1)
- `queue-type`: NonMix, PureCoinFlipping, PureRandomSampling, PermutedCoinFlipping, NoisyCoinFlipping, or NoisyCoinFlippingRandomRelease
- `outdir`: Output directory
- `num-threads`: The number of threads to run each iteration of the parameter sets of each [experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#ffbcb5071dbb482bad035ef01cf8d49d)

## Outputs

```
<outdir>/
  ordering_e1s1_PureCoinFlipping_2024-09-16T09:18:59.482141+00:00_0d0h0m10s/
    paramset_[1..P]/
      paramset.csv
      iteration_[0..I]_0d0h0m0s/
        topology.csv
        latency.csv
        data_msg_counts.csv
        sent_seq_[0..S].csv
        recv_seq_[0..R].csv
```
- `paramset_[1..P]/`: Each [experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#ffbcb5071dbb482bad035ef01cf8d49d) consists of multiple parameter sets. The result of each parameter set is stored in a separate directory.
- `paramset.csv`: The detailed parameters of the parameter set.
- `iteration_[0..I]_0d0h0m0s`: The result of each iteration (with the elapsed time)
  - `toplogy.csv`: The randomly generated topology of the network
  - `latency.csv`: The latency for each message to be delivered from the sender to the receiver
  - `data_msg_counts.csv`: The number of data messages staying in each queue in each time window
  - `sent_seq_[0..S].csv`: The sequence of sent messages by the sender
  - `sent_seq_[0..R].csv`: The sequence of received messages by the receiver

## Aggregation Tools

Since the ordering experiments are heavy, the aggregation tools are provided separately to aggregate the results of all experiments ans parameter sets after the experiments are done.

### Latency Aggregation

This tool reads all `**/iteration_*/latency.csv` files and aggregates all latencies into a single CSV file: `**/paramset_[1..P]/latency_stats.csv`, as below.
```csv
min,median,mean,std,max
0,93.0,123.07003891050584,109.38605760356785,527
```

### Data Message Counts Aggregation

This tool reads all `**/iteration_*/data_msg_counts.csv` files and aggregates all counts into a single CSV file: `**/paramset_[1..P]/data_msg_counts_stats.csv`, as below.
```csv
min,median,mean,std,max
0,1.0,9.231619223429988,31.290104671648148,289
```

### Ordering Coefficient Calculation

This tool is not an aggregation tool, but it calculates the [strong/casual/weak ordering coefficients](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#ee984d48bd6b4fe3b2acc1000e4ae77b)
from the `**/iteration_*/sent_seq_*.csv` and `**/iteration_*/recv_seq_*.csv` files.
The result is stored in CSV files: `**/iteration_*/coeffs_[sender_id]_[receiver_id].csv`, as below.
```csv
sender,receiver,strong,casual,weak
0,1,0,1,4
```

### Ordering Coefficient Aggregation

This tool reads all `**/iteration_*/coeffs_*_*.csv` files (calculated [above](#ordering-coefficient-calculation)) and aggregates all coefficients into three CSV file: `**/paramset_[1..P]/[strong|casual|weak]_coeff_stats.csv`, as below.
```csv
min,median,mean,std,max
0,0.0,0.25,0.4442616583193193,1
```

### Final Aggregation Across All Experiments

This tool reads all of the following files:
- `**/latency_stats.csv`
- `**/data_msg_counts_stats.csv`
- `**/[strong|casual|weak]_coeff_stats.csv`
and aggregates them into a single CSV file: `aggregated.csv`, as below.
```csv
paramset,num_mixes,num_paths,random_topology,peering_degree,min_queue_size,transmission_rate,num_senders,num_sender_msgs,sender_data_msg_prob,mix_data_msg_prob,queue_type,num_iterations,data_msg_count_min,data_msg_count_median,data_msg_count_mean,data_msg_count_std,data_msg_count_max,latency_min,latency_median,latency_mean,latency_std,latency_max,strong_coeff_min,strong_coeff_median,strong_coeff_mean,strong_coeff_std,strong_coeff_max,casual_coeff_min,casual_coeff_median,casual_coeff_mean,casual_coeff_std,casual_coeff_max,weak_coeff_min,weak_coeff_median,weak_coeff_mean,weak_coeff_std,weak_coeff_max
1,8,0,true,2,10,1,1,10000,0.01,0.0625,NoisyCoinFlipping,10,0,1.0,9.231619223429988,31.290104671648148,289,0,93.0,123.07003891050584,109.38605760356785,527,0,0.0,0.25,0.4442616583193193,1,0,0.0,0.45,0.6863327411532597,2,0,2.0,1.85,1.5312533566021211,5
...
```
This `aggregated.csv` file can be analyzed by the [analysis tools](../analysis).
