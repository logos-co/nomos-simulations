import os
import csv
import json
import random
import shutil
from build_config import build_config

def build_case(overlay, committees, nodes, config_name, max_view=1, network='default'):
    build_config(overlay, committees, nodes, config_name, max_view, network)
    # rename the runs with same configs 
    modified_name = config_name
    if os.path.exists(f"../configs/{config_name}.json"):
        tail = random.randint(1, 10000)
        modified_name = f"{config_name}_{tail}"
        print(f"name clash: renaming {config_name}.json to {modified_name}.json")
    with open(f"{config_name}.json", "r+") as f:
        data = json.load(f)
        data["stream_settings"]["path"] = f"output/{modified_name}.csv"
        f.seek(0)
        json.dump(data, f, indent=4)
        f.truncate()
    os.rename(f"{config_name}.json", f"{modified_name}.json")
    shutil.move(f"{modified_name}.json", "../configs/")

def build_cases(csv_path):
    with open(csv_path, 'r') as csv_file:
        reader = csv.reader(csv_file)
        for row in reader:
            overlay_type, node_count, committees, desc = row
            if overlay_type == "overlay":
                continue
            config_name = f"{overlay_type}_{node_count}_{committees}"
           # build_case(overlay_type, committees, node_count, f"{config_name}_view_1_default")
           # build_case(overlay_type, committees, node_count, f"{config_name}_view_10_default", max_view="5")
           # build_case(overlay_type, committees, node_count, f"{config_name}_view_10_optimistic", max_view="5", network="optimistic")
           # build_case(overlay_type, committees, node_count, f"{config_name}_view_10_pessimistic", max_view="5", network="pessimistic")
            build_case(overlay_type, committees, node_count, f"{config_name}_view_10_nolat", max_view="10", network="nolat")

if __name__ == "__main__":
    import sys
    if len(sys.argv) != 2:
        print("Usage: python generate_configs.py <path_to_csv_file>")
        sys.exit(1)

    csv_path = sys.argv[1]
    build_cases(csv_path)

