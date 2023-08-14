#!/bin/bash

if [ "$#" -lt 4 ]; then
    echo "Usage: $0 <overlay_type> <number_of_committees> <node_count> <config_name> [max_view] [network_config]"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "jq is not installed."
    exit 1
fi

if ! command -v sponge &> /dev/null; then
    echo "moreutils is not installed."
    exit 1
fi

OVERLAY_TYPE=$1
NUMBER_OF_COMMITTEES=$2
NODE_COUNT=$3
CONFIG_NAME=$4
MAX_VIEW=${5:-1}
NETWORK_CONFIG=${6:-default}

TEMPLATE_PATH="config_builder/template.json"
TEMPORARY_PATH="config_builder/temp.json"
NETWORK_UPDATE_PATH="config_builder/network/network_$NETWORK_CONFIG.json"
RECORD_UPDATE_PATH="config_builder/record.json"

# Update the template with the network settings
jq --slurpfile networkUpdate $NETWORK_UPDATE_PATH \
	'.network_settings = $networkUpdate[0].network_settings' $TEMPLATE_PATH | sponge $TEMPORARY_PATH

# Update the template with the record settings
jq --slurpfile recordUpdate $RECORD_UPDATE_PATH \
	'.record_settings = $recordUpdate[0].record_settings' $TEMPORARY_PATH | sponge $TEMPORARY_PATH

# Update new JSON with the command line arguments
jq --arg numCommit "$NUMBER_OF_COMMITTEES" \
	--arg nodeCount "$NODE_COUNT" \
	--arg confName "$CONFIG_NAME" \
	--arg maxView "$MAX_VIEW" \
	'.node_count = ($nodeCount | tonumber) | .stream_settings.path = "output/" + $confName + ".json" | .wards[0].max_view = ($maxView | tonumber)' $TEMPORARY_PATH | sponge $TEMPORARY_PATH

if [ "$OVERLAY_TYPE" == "tree" ]; then
	jq --arg numCommit "$NUMBER_OF_COMMITTEES" \
	'.overlay_settings.number_of_committees = ($numCommit | tonumber)' $TEMPORARY_PATH | sponge $TEMPORARY_PATH
elif [ "$OVERLAY_TYPE" == "branch" ]; then
	cat $TEMPORARY_PATH | jq --arg numCommit "$NUMBER_OF_COMMITTEES" \
	'.overlay_settings.branch_depth = ($numCommit | tonumber)' $TEMPORARY_PATH | sponge $TEMPORARY_PATH
else
	echo "Unknown overlay type. Supported types are 'tree' and 'branch'."
	exit 1
fi

mv $TEMPORARY_PATH "$CONFIG_NAME.json"
echo "Configuration built and saved as $CONFIG_NAME.json"
