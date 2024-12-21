import csv
import json
import os
import subprocess
import sys
from collections import OrderedDict
from dataclasses import asdict

import latency
import mixlog

STEP_DURATION_MS = 50


def bandwidth_result(log_path: str) -> dict[str, float]:
    max_step_id = 0
    for _, json_msg in mixlog.get_input_stream(log_path):
        if (step_id := json_msg.get("step_id")) is not None:
            max_step_id = max(max_step_id, step_id)

    with open(log_path, "r") as file:
        for line in file:
            if "total_outbound_bandwidth" in line:
                line = line[line.find("{") :]
                line = line.replace("{ ", '{"')
                line = line.replace(": ", '": ')
                line = line.replace(", ", ', "')
                record = json.loads(line)

                elapsed = (max_step_id * STEP_DURATION_MS) / 1000.0
                return {
                    "min": float(record["min_node_total_bandwidth"]) / elapsed,
                    "avg": float(record["avg_node_total_bandwidth"]) / elapsed,
                    "max": float(record["max_node_total_bandwidth"]) / elapsed,
                }

    raise Exception("No bandwidth data found in log file")


def topology_result(log_path: str) -> dict[str, int]:
    for topic, json_msg in mixlog.get_input_stream(log_path):
        if topic == "Topology":
            return json_msg
    raise Exception("No topology found in log file")


# Read the CSV data
csv_data = []
with open("params.csv", mode="r") as csvfile:  # Replace with your CSV file name
    reader = csv.DictReader(csvfile, delimiter=",")
    csv_data = list(reader)

# Read the original blendnet.json
with open("../config/blendnet.json", "r") as jsonfile:
    original_json = json.load(jsonfile)

# Directory to save modified JSON files
output_dir = "modified_configs"
os.makedirs(output_dir, exist_ok=True)

# Modify and save JSON files for each row in CSV
config_paths = []
for idx, row in enumerate(csv_data):
    modified_json = OrderedDict(original_json)  # Preserve original field order

    # Apply modifications
    modified_json["network_settings"]["regions"]["north america"] = 0.0
    modified_json["network_settings"]["regions"]["europe"] = 1.0
    modified_json["network_settings"]["regions"]["asia"] = 0.0
    modified_json["step_time"] = f"{STEP_DURATION_MS}ms"
    modified_json["node_count"] = int(row["network_size"])
    modified_json["wards"][0]["sum"] = 1000
    modified_json["connected_peers_count"] = int(row["peering_degree"])
    modified_json["data_message_lottery_interval"] = "20s"
    modified_json["stake_proportion"] = 0.0
    modified_json["persistent_transmission"]["max_emission_frequency"] = 1.0
    modified_json["persistent_transmission"]["drop_message_probability"] = 0.0
    modified_json["epoch_duration"] = (
        f"{int(row['cover_slots_per_epoch']) * int(row['cover_slot_duration'])}s"
    )
    modified_json["slots_per_epoch"] = int(row["cover_slots_per_epoch"])
    modified_json["slot_duration"] = f"{row['cover_slot_duration']}s"
    modified_json["max_delay_seconds"] = int(row["max_temporal_delay"])
    modified_json["number_of_hops"] = int(row["blend_hops"])
    modified_json["number_of_blend_layers"] = int(row["blend_hops"])

    # Save modified JSON
    output_path = os.path.join(output_dir, f"{idx}.json")
    with open(output_path, "w") as outfile:
        json.dump(modified_json, outfile, indent=2)
        print("Saved modified JSON to:", output_path)
    config_paths.append(output_path)

# Directory to save logs
log_dir = "logs"
os.makedirs(log_dir, exist_ok=True)

log_paths = []
for idx, config_path in enumerate(config_paths):
    log_path = f"{log_dir}/{idx}.log"
    with open(log_path, "w") as log_file:
        print(f"Running simulation-{idx}: {log_file.name} with config: {config_path}")
        subprocess.run(
            ["../../target/release/blendnet-sims", "--input-settings", config_path],
            stdout=log_file,
        )
        print(f"Simulation-{idx} completed: {log_file.name}")
    log_paths.append(log_path)


print("Analyzing logs...")
print("=================")

with open("output.csv", "w", newline="") as file:
    print(f"Writing results to: {file.name}")
    csv_writer = csv.writer(file)
    csv_writer.writerow(
        [
            "network_diameter",
            "msg_count",
            "min_latency",
            "avg_latency",
            "median_latency",
            "max_latency",
            "min_latency_msg_id",
            "min_latency_msg_persistent_latency_ms",
            "min_latency_msg_persistent_queue_sizes",
            "min_latency_msg_temporal_latency_ms",
            "min_latency_msg_temporal_queue_sizes",
            "max_latency_msg_id",
            "max_latency_msg_persistent_latency_ms",
            "max_latency_msg_persistent_queue_sizes",
            "max_latency_msg_temporal_latency_ms",
            "max_latency_msg_temporal_queue_sizes",
            "min_bandwidth_kbps",
            "avg_bandwidth_kbps",
            "max_bandwidth_kbps",
        ]
    )

    for idx, log_path in enumerate(log_paths):
        csv_row = []
        csv_row.append(topology_result(log_path)["diameter"])

        message_storage, node_storage = latency.parse_record_stream(
            mixlog.get_input_stream(log_path)
        )
        with open(f"{log_dir}/msgs-{idx}.json", "w") as file:
            json.dump(
                {msg_id: asdict(msg) for msg_id, msg in message_storage.items()},
                file,
                indent=2,
            )
        with open(f"{log_dir}/nodes-{idx}.json", "w") as file:
            json.dump(node_storage.to_dict(), file, indent=2)

        latency_analysis = latency.LatencyAnalysis.build(
            message_storage, node_storage, STEP_DURATION_MS
        )
        csv_row.append(latency_analysis.total_complete_messages)
        csv_row.append(float(latency_analysis.min_latency_ms) / 1000.0)
        csv_row.append(float(latency_analysis.avg_latency_ms) / 1000.0)
        csv_row.append(float(latency_analysis.median_latency_ms) / 1000.0)
        csv_row.append(float(latency_analysis.max_latency_ms) / 1000.0)
        csv_row.append(latency_analysis.min_latency_analysis.message_id)
        csv_row.append(
            ",".join(
                map(
                    str,
                    [
                        ms / 1000.0
                        for ms in latency_analysis.min_latency_analysis.persistent_latencies_ms
                    ],
                )
            )
        )
        csv_row.append(
            ",".join(
                map(str, latency_analysis.min_latency_analysis.persistent_queue_sizes)
            )
        )
        csv_row.append(
            ",".join(
                map(
                    str,
                    [
                        ms / 1000.0
                        for ms in latency_analysis.min_latency_analysis.temporal_latencies_ms
                    ],
                )
            )
        )
        csv_row.append(
            ",".join(
                map(str, latency_analysis.min_latency_analysis.temporal_queue_sizes)
            )
        )
        csv_row.append(latency_analysis.max_latency_analysis.message_id)
        csv_row.append(
            ",".join(
                map(
                    str,
                    [
                        ms / 1000.0
                        for ms in latency_analysis.max_latency_analysis.persistent_latencies_ms
                    ],
                )
            )
        )
        csv_row.append(
            ",".join(
                map(str, latency_analysis.max_latency_analysis.persistent_queue_sizes)
            )
        )
        csv_row.append(
            ",".join(
                map(
                    str,
                    [
                        ms / 1000.0
                        for ms in latency_analysis.max_latency_analysis.temporal_latencies_ms
                    ],
                )
            )
        )
        csv_row.append(
            ",".join(
                map(str, latency_analysis.max_latency_analysis.temporal_queue_sizes)
            )
        )

        bandwidth_res = bandwidth_result(log_path)
        csv_row.append(bandwidth_res["min"] * 8 / 1000.0)
        csv_row.append(bandwidth_res["avg"] * 8 / 1000.0)
        csv_row.append(bandwidth_res["max"] * 8 / 1000.0)

        csv_writer.writerow(csv_row)

    print(f"The outputs have been successfully written to {file.name}")
