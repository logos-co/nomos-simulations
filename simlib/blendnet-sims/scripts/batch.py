import csv
import json
import os
import subprocess
from collections import OrderedDict

import latency
import mixlog

STEP_DURATION_MS = 50


def bandwidth_result(log_path: str) -> dict[str, float]:
    step_id = 0

    for line in mixlog.get_input_stream(log_path):
        if '"payload_id"' in line and '"step_id"' in line:
            step_id = max(step_id, json.loads(line)["step_id"])

        if "total_outbound_bandwidth" in line:
            line = line.replace("{ ", '{"')
            line = line.replace(": ", '": ')
            line = line.replace(", ", ', "')
            record = json.loads(line)

            elapsed = (step_id * STEP_DURATION_MS) / 1000.0
            return {
                "min": float(record["min_node_total_bandwidth"]) / elapsed,
                "avg": float(record["avg_node_total_bandwidth"]) / elapsed,
                "max": float(record["max_node_total_bandwidth"]) / elapsed,
            }

    raise Exception("No bandwidth data found in log file")


def topology_result(log_path: str) -> dict[str, int]:
    for line in mixlog.get_input_stream(log_path):
        if "longest_path_len" in line:
            return json.loads(line)
    raise Exception("No bandwidth data found in log file")


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
    modified_json["step_time"] = f"{STEP_DURATION_MS}ms"
    modified_json["node_count"] = int(row["network_size"])
    modified_json["connected_peers_count"] = int(row["peering_degree"])
    modified_json["epoch_duration"] = (
        f"{int(row['cover_slots_per_epoch']) * int(row['cover_slot_duration'])}s"
    )
    modified_json["slots_per_epoch"] = int(row["cover_slots_per_epoch"])
    modified_json["slot_duration"] = f"{row['cover_slot_duration']}s"
    modified_json["max_delay_seconds"] = int(row["max_temporal_delay"])
    modified_json["number_of_hops"] = int(row["blend_hops"])
    modified_json["number_of_blend_layers"] = int(row["blend_hops"])

    # Save modified JSON
    output_path = os.path.join(output_dir, f"blendnet-{idx}.json")
    with open(output_path, "w") as outfile:
        json.dump(modified_json, outfile, indent=4)
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
print(
    "longest_path_len,msg_count,min_latency_msg_id,max_latency_msg_id,min_latency,avg_latency,median_latency,max_latency,min_bandwidth_kbps,avg_bandwidth_kbps,max_bandwidth_kbps"
)
for idx, log_path in enumerate(log_paths):
    longest_path_len = topology_result(log_path)["longest_path_len"]
    latency_res = latency.compute_results(
        latency.parse_record_stream(mixlog.get_input_stream(log_path)), STEP_DURATION_MS
    )
    msg_count = latency_res["total_complete_messages"]
    min_latency = float(latency_res["min_latency_ms"]) / 1000.0
    min_latency_msg_id = latency_res["min_latency_message_id"]
    avg_latency = float(latency_res["latency_average_ms"]) / 1000.0
    median_latency = float(latency_res["latency_median_ms"]) / 1000.0
    max_latency = float(latency_res["max_latency_ms"]) / 1000.0
    max_latency_msg_id = latency_res["max_latency_message_id"]

    bandwidth_res = bandwidth_result(log_path)
    min_bandwidth_kpbs = bandwidth_res["min"] * 8 / 1000.0
    avg_bandwidth_kpbs = bandwidth_res["avg"] * 8 / 1000.0
    max_bandwidth_kpbs = bandwidth_res["max"] * 8 / 1000.0

    print(
        f"{longest_path_len},{msg_count},{min_latency_msg_id},{max_latency_msg_id},{min_latency},{avg_latency},{median_latency},{max_latency},{min_bandwidth_kpbs},{avg_bandwidth_kpbs},{max_bandwidth_kpbs}"
    )
