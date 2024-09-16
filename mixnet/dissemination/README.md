# Queuing Mechanism: Message Dissemination Experiments

This directory contains the code for the [Message Dissemination Experiments](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#5120661bdf5343319e66c9372dd623b7), which is the part of the Queuing Mechanism Experiments.

## Usages

```bash
cargo install --path dissemination

dissemination --exp-id 1 --session-id 1 --queue-type PureCoinFlipping --outdir $PWD/out --num-threads 4
```
- `exp-id`: [Experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#4543053fbb8c4a2f8d49b0dffdb4a928) ID (starting from 1)
- `session-id`: [Session](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#ced2155214f442ed95b442c18f5832f6) ID (starting from 1)
- `queue-type`: NonMix, PureCoinFlipping, PureRandomSampling, PermutedCoinFlipping, NoisyCoinFlipping, or NoisyCoinFlippingRandomRelease
- `outdir`: Output directory
- `num-threads`: The number of threads to run each iteration of the parameter sets of each [experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#4543053fbb8c4a2f8d49b0dffdb4a928)

## Outputs

```
<outdir>/
  dissemination_e1s1_PureCoinFlipping_2024-09-16T09:09:08.793730+00:00_0d0h12m30s/
    paramset_[1..P]/
      paramset.csv
      iteration_[0..I].csv
      topology_[0..I].csv
```
- `paramset_[1..P]/`: Each [experiment](https://www.notion.so/Nomos-Mix-Queueing-Mechanism-Experimentation-Methodology-d629af5a2d43473c9ec9ba191f6d904d?pvs=4#4543053fbb8c4a2f8d49b0dffdb4a928) consists of multiple parameter sets. The result of each parameter set is stored in a separate directory.
- `paramset.csv`: The detailed parameters of the parameter set.
- `iteration_[0..I].csv`: The result of each iteration.
  - Columns
    - `dissemination_time`: The time taken to disseminate the message to the entire network
    - `sent_time`: The time when the message was sent by a sender
    - `all_received_time`: The time when the message was received by all nodes
  - `dissemination_time = all_received_time - sent_time`
  - All times used in the simulations are virtual discrete time units.
- `topology_[0..I].csv`: The randomly generated network topology of each iteration
