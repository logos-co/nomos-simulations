import argparse

import usim

from sim.config import Config
from sim.simulation import Simulation

if __name__ == "__main__":
    """
    Read a config file and run a simulation
    """
    parser = argparse.ArgumentParser(
        description="Run mixnet simulation",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "--config", type=str, required=True, help="Configuration file path"
    )
    args = parser.parse_args()

    config = Config.load(args.config)
    sim = Simulation(config)
    usim.run(sim.run())

    print("Simulation complete!")
