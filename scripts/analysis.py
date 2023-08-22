import os
import sys
import math
import typer
import json
import pandas as pd
import numpy as np
import logging as log
from pathlib import Path
import matplotlib.pyplot as plt
from os import walk
from collections import defaultdict

def read_json(fname):
    with open(fname) as f:
        cdata = json.load(f)
    return cdata


def read_csv(fname):
    df = pd.read_csv(fname, header=0,  comment='#', skipinitialspace = True )
    return df


def write_csv(df, fname):
    df.to_csv(fname)


def compute_view_finalisation_times(df, conf, oprefix, tag="tag", plot=False, epoch=True):
    num_nodes = conf["node_count"]
    two3rd = math.floor(num_nodes * 2/3) + 1

    views, view2fin_time = df.current_view.unique()[:-2], {}
    log.debug(f'views: {views}')

    for start_view in views:
        end_view = start_view + 2
        start_idx = df.index[(df['current_view'] == start_view)][0]
        end_idx = df.index[(df['current_view'] == end_view)][two3rd-1]
        start_step = df.iloc[start_idx].step_id
        end_step = df.iloc[end_idx].step_id
        view2fin_time[start_view] =  end_step - start_step
        log.debug(f'{start_view}({start_idx}), {end_view}({end_idx}) : {end_step} - {start_step} = {view2fin_time[start_view]}')

    if not plot:
        if not view2fin_time:
            return conf["views_count"]
        return sum(view2fin_time.values())/len(view2fin_time.values())

    fig, axes = plt.subplots(1, 1, layout='constrained', sharey=False)
    fig.set_figwidth(12)
    fig.set_figheight(10)

    fig.suptitle(f'View Finalisation Times - {tag}')
    axes.set_ylabel("Number of Epochs to Finalise a View")
    axes.set_xlabel("Views")
    axes.set_xticks([x for x in view2fin_time.keys()])
    axes.set_yticks([x for x in range(0, max(view2fin_time.values())+2)])

    axes.plot(view2fin_time.keys(), view2fin_time.values(), linestyle='--', marker='o')
    plt.show()
    plt.savefig(f'{oprefix}-view-finalisation-times.pdf', format="pdf", bbox_inches="tight")


app = typer.Typer()

@app.command()
def view(ctx: typer.Context,
        data_file: Path = typer.Option("config.json",
                exists=True, file_okay=True, readable=True,
                help="Set the simulation config file"),
        config_file: Path = typer.Option("simout.csv",
                exists=True, file_okay=True, readable=True,
                help="Set the simulation data file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots"),
        ):
    log.basicConfig(level=log.INFO)

    tag = os.path.splitext(os.path.basename(data_file))[0]
    conf, df = read_json(config_file), read_csv(data_file)
    compute_view_finalisation_times(df, conf, oprefix, tag, plot=True, epoch=True)

@app.command()
def views(ctx: typer.Context,
        path: Path = typer.Option("../",
                exists=True, dir_okay=True, readable=True,
                help="Set the simulation config file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots")
        ):

    tag2vfins = {}
    conf_fnames = next(walk(f'{path}/configs'), (None, None, []))[2]  # [] if no file
    for conf in conf_fnames:
        tag = os.path.splitext(os.path.basename(conf))[0]
        tag2vfins[tag] = {}
        conf, df = read_json(f'{path}/configs/{conf}'), read_csv(f'{path}/output/output/{tag}.csv')
        simtype = conf["stream_settings"]["path"].split("/")[1].split("_")[0]

        res = compute_view_finalisation_times(df, conf, oprefix, tag, plot=False, epoch=True)
        if simtype == "tree":
            num_nodes = conf["node_count"]
            tag2vfins[tag][num_nodes] = res
        else:
            max_depth = conf["overlay_settings"]["branch_depth"]
            tag2vfins[tag][max_depth] = res
        print(tag, res)
    print(tag2vfins)

@app.command()
def other_commands():
    pass


if __name__ == "__main__":
    app()



