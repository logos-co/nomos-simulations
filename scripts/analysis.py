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

def compute_view_finalisation_times(df, conf, oprefix, simtype, tag="tag", plot=False):
    if simtype == "tree":
        num_nodes = conf["node_count"]
    else:
        num_tree_nodes = 2 ** (conf["overlay_settings"]["branch_depth"]) - 1
        num_committees = int (conf["node_count"]/conf["overlay_settings"]["branch_depth"])
        num_nodes = num_tree_nodes * num_committees
        log.debug(f"num nodes: {num_nodes, num_tree_nodes, num_committees}")

    two3rd = math.floor(conf["node_count"] * 2/3) + 1

    views, view2fin_time = df.current_view.unique()[:-2], {}
    log.info(f'views: {conf["stream_settings"]["path"]} {views},  {df.current_view.unique()}')

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

def compute_view_times(path, oprefix):
    nwsize2vfins = {}
    conf_fnames = next(walk(f'{path}/configs'), (None, None, []))[2]
    for conf in conf_fnames:
        tag = os.path.splitext(os.path.basename(conf))[0]
        cfile, dfile =  f'{path}/configs/{conf}', f'{path}/output/{tag}.csv'
        conf, df = read_json(cfile), read_csv(dfile)
        simtype = conf["stream_settings"]["path"].split("/")[1].split("_")[0]
        view2fin = compute_view_finalisation_times(df, conf, oprefix, simtype, tag, plot=False)
        if not view2fin:  # < 2 views
            continue
        if simtype == "tree":
            num_nodes = conf["node_count"]
        else:
            num_tree_nodes = 2 ** (conf["overlay_settings"]["branch_depth"]) - 1
            num_committees = int (conf["node_count"]/conf["overlay_settings"]["branch_depth"])
            num_nodes = num_tree_nodes * num_committees
        #num_nodes = conf["node_count"]
        if simtype == "branch":
            max_depth = conf["overlay_settings"]["branch_depth"]
        else:
            max_depth =  math.log(num_nodes + 1, 2) - 1
        if num_nodes in nwsize2vfins:
            nwsize2vfins[num_nodes].append((simtype, max_depth, view2fin, tag))
        else:
            nwsize2vfins[num_nodes] = [(simtype, max_depth, view2fin, tag)]
    return nwsize2vfins

def plot_view_times(nwsize2vfins, simtype, oprefix):
    logbands = {}
    logbands[simtype] = {}
    logbands[simtype]["low"] = []
    logbands[simtype]["high"] = []

    if simtype == "branch":
        low, high = 0.75, 1.5
    else:
        low, high = 0.75, 1.5
    data = [[], []]
    for n in sorted(list(map(int, nwsize2vfins.keys()))):
        vfin = nwsize2vfins[n]
        print(f"{simtype} {nwsize2vfins[n]}",  end=' == ')
        for  run in vfin:
            if "nolat" in run[3] and simtype in run[0]:
                data[0].append(n)
                data[1].append(int(run[2][0]))
                print(f"IF: {simtype}={run[0]} :  {n} {run[3]}")
                logbands[simtype]["low"].append(math.log(n, 2)*low)
                logbands[simtype]["high"].append(math.log(n, 2)*high)
            else:
                print(f"ELSE: {simtype}!={run[0]} :  {n} {run[3]}")


    print(data)
    fig, axes = plt.subplots(1, 1, layout='constrained', sharey=False)
    fig.set_figwidth(12)
    fig.set_figheight(10)

    fig.suptitle(f'View Finalisation Times - {simtype}')
    axes.set_ylabel("Number of Epochs")
    axes.set_xlabel("Number of Nodes")

    l1 = axes.plot(data[0], data[1], linestyle='-', marker='o', label='Carnot')
    l2 = axes.plot(data[0], logbands[simtype]["low"], linestyle='--', marker='x', label=f'{low} * log(#nodes)')
    l3 = axes.plot(data[0], logbands[simtype]["high"], linestyle='--', marker='x', label=f'{high} * log(#nodes)')
    l = l1 + l2 + l3

    labels = [curve.get_label() for curve in l]
    axes.legend(l, labels, loc="lower right")

    plt.show()
    plt.savefig(f'{oprefix}-{simtype}-output.pdf', format="pdf", bbox_inches="tight")
    plt.clf()
    plt.cla()
    plt.close()
    return data

def plot_tree_vs_branch(tree, branch, oprefix):

    print(tree, branch)
    fig, axes = plt.subplots(1, 1, layout='constrained', sharey=False)
    fig.set_figwidth(12)
    fig.set_figheight(10)

    fig.suptitle(f'View Finalisation Times - Tree vs Branch')
    axes.set_xlabel("Tree")
    axes.set_ylabel("Branch")


    #print("\nT, B:", tree[1], branch[1])
    branch[1].insert(0, 6)
    print("\nT, B:", tree[1], branch[1])
    axes.scatter(tree[1], branch[1])

    axes.plot([0, 1], [0, 1],  linestyle='dashed', transform=axes.transAxes)

    '''
    fig.suptitle(f'View Finalisation Times - Tree vs Branch')
    axes.set_xlabel("Number of Nodes")
    axes.set_ylabel("Number of Epochs")


    axes.scatter(tree[0], tree[1], label="Tree")
    axes.scatter(branch[0], branch[1], label="Branch")
    '''

    plt.show()
    plt.savefig(f'{oprefix}-scatter.pdf', format="pdf", bbox_inches="tight")
    plt.clf()
    plt.cla()
    plt.close()


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
    compute_view_finalisation_times(df, conf, oprefix, simtype, tag, plot=True)

@app.command()
def views(ctx: typer.Context,
        path: Path = typer.Option("../",
                exists=True, dir_okay=True, readable=True,
                help="Set the simulation config file"),
        oprefix: str = typer.Option("output",
                help="Set the output prefix for the plots")
        ):

    log.basicConfig(level=log.INFO)
    nwsize2vfins = compute_view_times(path, oprefix)
    write_dict(nwsize2vfins, f'{oprefix}-viewtimes.dict')

    print("reading ")
    nwsize2vfins = read_dict(f'{oprefix}-viewtimes.dict')
    tree = plot_view_times(nwsize2vfins, "tree", oprefix)
    branch = plot_view_times(nwsize2vfins, "branch", oprefix)

    plot_tree_vs_branch(tree, branch, oprefix)

@app.command()
def other_commands():
    pass


if __name__ == "__main__":
    app()



