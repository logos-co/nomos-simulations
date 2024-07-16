import subprocess
import argparse
import os
import time

def run_simulation(command):
    start_time = time.time()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1, universal_newlines=True)
    process.communicate()

def run_simulations(configs_path):
    for filename in os.listdir(configs_path):
        if os.path.isfile(os.path.join(configs_path, filename)):
            print(f"Starting {filename}")
            run_simulation(["simulation", "--input-settings", f"{configs_path}/{filename}", "--stream-type", "naive"])
            print(f"Finished {filename}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run simulations for all configs in the provided directory")
    parser.add_argument("configs_path", type=str, help="The string to search for in the command's output.")

    args = parser.parse_args()
    run_simulations(args.configs_path)
