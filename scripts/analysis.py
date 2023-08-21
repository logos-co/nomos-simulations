import os
import typer
import json
import pandas as pd
import numpy as np
import logging as log
from pathlib import Path
import matplotlib.pyplot as plt

def read_json(fname):
    with open(fname) as f:
        cdata = json.load(f)
    return cdata


def read_csv(fname):
    df = pd.read_csv(fname, header=0,  comment='#', skipinitialspace = True )
    return df


def write_csv(df, fname):
    df.to_csv(fname)


app = typer.Typer()

@app.command()
def views(ctx: typer.Context,
        data_file: Path = typer.Option("simout.csv",
                exists=True, file_okay=True, readable=True,
                help="Set the simulation data file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots"),
        debug: bool = typer.Option(True,
                help="Set debug")
        ):
    log.basicConfig(level=log.INFO)
    tag = os.path.splitext(os.path.basename(data_file))[0]

    df = read_csv(data_file)
    steps_df = data=df.drop_duplicates('current_view').step_id.diff().values[1:]
    views_df  = df['current_view'].unique()[1:]

    fig, axes = plt.subplots(1, 1, layout='constrained', sharey=False)
    fig.set_figwidth(12)
    fig.set_figheight(10)

    fig.suptitle(f'View installation times :: {tag}')
    axes.set_ylabel("Number of Epochs")
    axes.set_xlabel("Views")
    axes.set_xticks([x + 1  for x in range(max(views_df.astype(int)))])
    axes.set_yticks([x + 1 for x in range(max(steps_df.astype(int)))])

    axes.plot(views_df, steps_df, linestyle='--', marker='o')
    plt.show()
    plt.savefig(f'{oprefix}-view-installion-times.pdf', format="pdf", bbox_inches="tight")


@app.command()
def other_commands():
    pass


if __name__ == "__main__":
    app()



