import os
import csv
import shutil
from build_config import build_config

def build_case(overlay, committees, nodes, config_name, max_view=1, network='default'):
    build_config(overlay, committees, nodes, config_name, max_view, network)
    if os.path.exists(f"../configs/{config_name}.json"):
        os.rename(f"{config_name}.json", f"{config_name}1.json")
        config_name=f"{config_name}1"
        print(config_name)
    shutil.move(f"{config_name}.json", "../configs/")

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

