import pandas as pd
import json
import argparse
import os


def data_to_csv(data_path, output_path):
    header_written = False

    with open(data_path, 'r') as f:
        step = 0
        for line in f:
            try:
                clean_line = line.rstrip(",\n")
                data = json.loads(clean_line)
                normalized = pd.json_normalize(data)
                normalized['step'] = step
                normalized.to_csv(output_path, mode='a', header=not header_written, index=False)
                
                # Set the header_written flag to True after the first write
                header_written = True
            except json.JSONDecodeError:
                print(f"Failed to parse line: {line}")
            step += 1

def all_data_to_csv(all_data_path):
    for filename in os.listdir(all_data_path):
        config_name = os.path.splitext(filename)[0]
        data_to_csv(f"{all_data_path}/{config_name}.json", f"{all_data_path}/{config_name}.csv")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Normalize JSON lines in a file to a Pandas DataFrame and append to CSV.")
    parser.add_argument("data_path", type=str, help="Path to the file containing JSON lines.")
    
    args = parser.parse_args()
    all_data_to_csv(args.data_path)
