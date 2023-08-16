import typer
import logging as log
from pathlib import Path

def read_csv(sim_dfile):
    df=None
    return df

def write_csv(df):
    pass


def plot_vtimes(df, oprefix=""):
    pass


def compute_vtimes(df):
    pass


def main(ctx: typer.Context,
        config_file: Path = typer.Option("config.json",
                exists=True, file_okay=True, readable=True,
                help="Set the config file"),
        data_file: Path = typer.Option("simout.csv",
                exists=True, file_okay=True, readable=True,
                help="Set the simulation data file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots"),
        debug: bool = typer.Option(True,
                help="Set debug")
        ):
    log.info(config_file, data_file, oprefix, debug)


if __name__ == "__main__":
    typer.run(main)
