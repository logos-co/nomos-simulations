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

def write_json(dic, fname):
    dic_str = {str(k): str(v) for k,v in dic.items()}
    jdump = json.dumps(dic_str, indent=4, sort_keys=True)
    with open(fname, 'w') as f:
        f.write(jdump)

def read_dict(fname):
    with open(fname, 'r') as f:
        return eval(f.read())

def write_dict(dic, fname):
    with open(fname, 'w') as f:
        return f.write(str(dic))

def read_csv(fname):
    df = pd.read_csv(fname, header=0,  comment='#', skipinitialspace = True )
    return df


def write_csv(df, fname):
    df.to_csv(fname)

def compute_view_finalisation_times(df, conf, oprefix, tag="tag", plot=False):
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
        return view2fin_time

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

def compute_view_lengths(path, oprefix):
    pathlen2vfins = {}
    conf_fnames = next(walk(f'{path}/configs'), (None, None, []))[2]
    for conf in conf_fnames:
        tag = os.path.splitext(os.path.basename(conf))[0]
        cfile, dfile =  f'{path}/configs/{conf}', f'{path}/output/output/{tag}.csv'
        conf, df = read_json(cfile), read_csv(dfile)
        simtype = conf["stream_settings"]["path"].split("/")[1].split("_")[0]
        view2fin = compute_view_finalisation_times(df, conf, oprefix, tag, plot=False)
        if not view2fin:  # < 2 views
            continue
        num_nodes = conf["node_count"]
        if simtype == "branch":
            max_depth = conf["overlay_settings"]["branch_depth"]
        else:
            max_depth =  math.log(num_nodes + 1, 2) - 1
        if num_nodes in pathlen2vfins:
            pathlen2vfins[num_nodes].append((simtype, max_depth, view2fin, tag))
        else:
            pathlen2vfins[num_nodes] = [(simtype, max_depth, view2fin, tag)]
    return pathlen2vfins


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
    compute_view_finalisation_times(df, conf, oprefix, tag, plot=True)

@app.command()
def views(ctx: typer.Context,
        path: Path = typer.Option("../",
                exists=True, dir_okay=True, readable=True,
                help="Set the simulation config file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots")
        ):

    log.basicConfig(level=log.INFO)
    #pathlen2vfins = compute_view_lengths(path, oprefix)
    #write_json(pathlen2vfins, f'{oprefix}-viewtimes.json')
    #write_dict(pathlen2vfins, f'{oprefix}-viewtimes.dict')

    pathlen2vfins = read_dict(f'{oprefix}-viewtimes.dict')
    data, log4, log5 = [[], []], [], []
    print("READ FROM FILE", pathlen2vfins)
    for n in sorted(list(map(int, pathlen2vfins.keys()))):
        vfin = pathlen2vfins[n]
        for  run in vfin:
            if "default" in run[3] and "tree" in run[0]:
                data[0].append(n)
                data[1].append(int(run[2][0]))
                log4.append(int(run[1])*1)
                log5.append(int(run[1])*2)

    fig, axes = plt.subplots(1, 1, layout='constrained', sharey=False)
    fig.set_figwidth(12)
    fig.set_figheight(10)

    fig.suptitle(f'View Finalisation Times')
    axes.set_ylabel("Number of Epochs")
    axes.set_xlabel("Number of Nodes")

    l1 = axes.plot(data[0], data[1], linestyle='-', marker='o', label='Carnot')
    l2 = axes.plot(data[0], log4, linestyle='--', marker='x', label='1 * log(#nodes)')
    l3 = axes.plot(data[0], log5, linestyle='--', marker='x', label='2 * log(#nodes)')
    l = l1 + l2 + l3

    labels = [curve.get_label() for curve in l]
    axes.legend(l, labels, loc="lower right")

    plt.show()
    plt.savefig(f'{oprefix}-output.pdf', format="pdf", bbox_inches="tight")


@app.command()
def other_commands():
    pass


if __name__ == "__main__":
    app()



