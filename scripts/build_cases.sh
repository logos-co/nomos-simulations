#!/bin/bash

# Check if the input file is provided
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <path_to_csv_file>"
    exit 1
fi

# Read the CSV file line by line
while IFS=, read -r overlay_type node_count committees; do
    if [[ "$overlay_type" == "overlay" ]]; then
        continue
    fi

	CONFIG_NAME="${overlay_type}_${node_count}_${committees}"
    ./build_config.sh "$overlay_type" "$committees" "$node_count" $CONFIG_NAME"_view_1_default"
    ./build_config.sh "$overlay_type" "$committees" "$node_count" $CONFIG_NAME"_view_10_default" 10
    ./build_config.sh "$overlay_type" "$committees" "$node_count" $CONFIG_NAME"_view_10_optimistic" 10 optimistic
    ./build_config.sh "$overlay_type" "$committees" "$node_count" $CONFIG_NAME"_view_10_pessimistic" 10 pessimistic
	mv *.json ../configs/
done < $1

