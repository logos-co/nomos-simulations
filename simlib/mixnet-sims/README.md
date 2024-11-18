# Nomos Blend Simulation

## Features

- The simulation runs multiple Nomos Blend nodes using [netrunner](../netrunner).
- Each node implements the Nomos Blend Tier 1~3 protocols.
    - [Persistent Transmission](https://www.notion.so/Nomos-Blend-Network-Tier-1-Persistent-Transmission-Module-10b8f96fb65c807cb1e8f92a7f41a771?pvs=4)
    - [Message Blending](https://www.notion.so/Nomos-Blend-Network-Tier-2-Message-Blending-Module-1208f96fb65c80e5bcb9df6e27472339?pvs=4)
    - [Cover Traffic](https://www.notion.so/Nomos-Blend-Network-Tier-3-Cover-Traffic-Module-10b8f96fb65c80cab153de10115e0023?pvs=4)
    - Also, each node generates data messages probabilistically (similar as the consensus protocol).
- The simulation prints logs that can be used to analyze the following properties.
    - Latency
    - Bandwidth
    - Anonymity

## Configurations

The simulation can be configured by [config/mixnode.json](./config/mixnode.json).

### `step_time` and `network_settings`

We recommend setting the step time equal to the minimum latency in the network settings,
in order to speed up the simulation.
For example, if all regions are used, and if the minimum latency is 40ms (asia:asia),
we recommend setting the step time to 40ms.

### `wards.sum`

The simulation runs until the `wards.sum` number of messages (data or cover) arrive in
their last blend node and finally fully unwrapped.

### Network Topology

The simulation constructs a network with the `node_count` number of nodes.
Each node establish connections with the `connected_peers_count` number of nodes randomly selected.

### Protocol Parameters

#### Tier 1: Persistent Transmission

```json
"persistent_transmission": {
  "max_emission_frequency": 1.0,
  "drop_message_probability": 0.0
},
```
We recommend setting `max_emission_frequency` to [1 message per second](https://www.notion.so/Nomos-Blend-Network-Tier-1-Persistent-Transmission-Module-10b8f96fb65c807cb1e8f92a7f41a771?pvs=4#11f8f96fb65c80dfbd23e4400feaaf9c),
which is the same as the expected maximum message frequency configured in the consensus protocol
that will use the Nomos Blend protocol.

To disable [drop messages](https://www.notion.so/Nomos-Blend-Network-Tier-1-Persistent-Transmission-Module-10b8f96fb65c807cb1e8f92a7f41a771?pvs=4#11c8f96fb65c804db7ccfd024f8c44d0), set `drop_message_probability` to 0.

#### Tier 2: Message Blending

```json
"number_of_mix_layers": 2,
"max_delay_seconds": 10
```

The `number_of_mix_layers` is a parameter for the [Cryptographic Processor](https://www.notion.so/Nomos-Blend-Network-Tier-2-Message-Blending-Module-1208f96fb65c80e5bcb9df6e27472339?pvs=4#1208f96fb65c80f8a8d3d2b449953bde). And, the `max_delay_seconds` is for [Temporal Processor](https://www.notion.so/Nomos-Blend-Network-Tier-2-Message-Blending-Module-1208f96fb65c80e5bcb9df6e27472339?pvs=4#1208f96fb65c80dca885dda33fbd599b), which is the same as $\Delta_{max}$ in the specification.

#### Tier 3: Cover Traffic

```json
"epoch_duration": "432000s",
"slot_duration": "20s",
"slots_per_epoch": 21600,
"number_of_hops": 2,
```
These parameters are defined in the [specification](https://www.notion.so/Nomos-Blend-Network-Tier-3-Cover-Traffic-Module-10b8f96fb65c80cab153de10115e0023?pvs=4#12f8f96fb65c80d094b6f31306c65b70).
In short, each node selects the `slots_per_epoch / node_count * (1 / number_of_hops)` number of slots
at the beginning of each epoch. At every selected slot, the node generates a cover message.


## Running the simulation

```bash
cargo build --release
../target/release/mixnet-sims --input-settings ./config/mixnode.json
```
The simulation prints a bunch of logs that can be used for analysis.
We recommend piping logs to a file.

## Analysis

To analysis logs for latency and anonymity, see the README in the [scripts](../../scripts/).

The bandwidth consumed by each node is printed as a log at the end of the simulation.
