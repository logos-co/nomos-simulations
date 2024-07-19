import sys
import os
import json

TEMPLATE_PATH = "config_builder/template.json"
TEMPORARY_PATH = "config_builder/temp.json"
NETWORK_UPDATE_PATH = "config_builder/network"
RECORD_UPDATE_PATH = "config_builder/record.json"

def build_config(overlay_type, number_of_committees, node_count, config_name, max_view=1, network='default'):

    with open(TEMPLATE_PATH, 'r') as f:
        data = json.load(f)

    network_file = f"{NETWORK_UPDATE_PATH}/network_{network}.json"
    with open(network_file, 'r') as f:
        network_update = json.load(f)
    data["network_settings"] = network_update["network_settings"]

    with open(RECORD_UPDATE_PATH, 'r') as f:
        record_update = json.load(f)
    data["record_settings"] = record_update["record_settings"]

    data["node_count"] = int(node_count)
    data["stream_settings"]["path"] = f"output/{config_name}.json"
    data["wards"][0]["max_view"] = int(max_view)

    if overlay_type == "tree":
        data["overlay_settings"]["number_of_committees"] = int(number_of_committees)
    elif overlay_type == "branch":
        data["overlay_settings"]["branch_depth"] = int(number_of_committees)
    else:
        print("Unknown overlay type. Supported types are 'tree' and 'branch'.")
        return

    with open(f"{config_name}.json", 'w') as f:
        json.dump(data, f, indent=4)

    print(f"Configuration built and saved as {config_name}.json")

if __name__ == "__main__":
    if len(sys.argv) < 5:
        print("Usage: python config_builder.py <overlay_type> <number_of_committees> <node_count> <config_name> [max_view] [network_config]")
        sys.exit(1)

    overlay_type = sys.argv[1]
    number_of_committees = sys.argv[2]
    node_count = sys.argv[3]
    config_name = sys.argv[4]
    max_view = sys.argv[5] if len(sys.argv) > 5 else 1
    network_config = sys.argv[6] if len(sys.argv) > 6 else 'default'

    build_config(overlay_type, number_of_committees, node_count, config_name, max_view, network_config)

