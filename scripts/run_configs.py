import subprocess
import argparse
import os
import time

def run_simulation(command):
    # TODO: Graceful shutdown in simulation doesn't work yet, remove the ouput filtering once implemented.
    search_string = "ALL DONE"

    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1, universal_newlines=True) as process:
        for line in iter(process.stdout.readline, ''):
            if search_string in line:
                # TODO: In simulation, subscriber will drop all remaining unpersisted records when the process
                # is terminated, add some delay for filesystem to catch up with writing the output data.
                time.sleep(5)
                process.terminate()
                return

        process.communicate()  # wait for the process to finish if it hasn't yet

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
