#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
params=[["n_M", 50], ["d_M", 1500], ["c_r", 31000], ["c_f", 3], ["c_e", 100]]
graph_path="data/aachen_net/aachen_graph"
import csv
import json
import logging
import math
from math import sqrt
from pathlib import Path

import networkx as nx
from networkx.readwrite import json_graph

import cplex
from docplex.mp.model import Model

logger = logging.getLogger('aachen_net.org')
logger.setLevel(logging.INFO)
logger.propagate = False

formatter = logging.Formatter("%(asctime)s::%(levelname)s::%(module)s::%(message)s",
                              "%Y-%m-%d %H:%M:%S")

ch = logging.StreamHandler()
ch.setLevel(logging.INFO)
ch.setFormatter(formatter)
logger.addHandler(ch)

logger.info('import ok')

def ILP_solve(G, params, logfile="logs/unnamed.log", minimum_gap=1e-4):
    '''
    G needs to have this properties
    - n_lines != 0 only for active nodes
    - distance for all edges

    Output will provide another graph with
    - edges
      - x, n: activity flag and number of lines passing
      - active: activity boolean (as heuristic)
      - n_lines: number of lines served of subroot (0 if not subroot)
    - nodes
      - d: distance from subroot (if active)
      - is_subroot: flag for subroots
      - active: activity boolean
    '''

    #####################
    # Pre-process graph #
    #####################
    G = G.to_directed()

    # add artificial root node to G, with a zero-length arc for all the nodes
    G.add_node('r', n_lines=0, lat=-1, lon=0)

    for node_id in G.nodes:
        # TODO check if node is in R (candidate sub-roots)
        G.add_edge('r', node_id, length=0)

    ###################
    # Setup variables #
    ###################

    m = Model(log_output=True)
    m.parameters.mip.tolerances.mipgap = minimum_gap
    m.parameters.workmem = 2048
    m.parameters.mip.display = 2
    m.parameters.mip.interval = -1

    def name(source, target=None, var='x'):
        if target:
            return "{}_{}~{}".format(var, source, target)
        else:
            return "{}_{}".format(var, source)

    X = {}
    N = {}
    for i, (source, target) in enumerate(G.edges):
        if i % 10000 == 0:
            print("Initializing edge {}/{}".format(i, len(G.edges)), end='\r')

        if source not in X:
            X[source] = {}

        if source not in N:
            N[source] = {}

        ## active edge indicator
        X[source][target] = m.binary_var(name=name(source, target, var='x'))
        N[source][target] = m.integer_var(name=name(source, target, var='n'))
        m.add_constraint(ct=N[source][target] >= 0,
                         ctname="{} >= 0".format(name(source, target, var='n')))

    D = {}
    for i, (node_id, data) in enumerate(G.nodes(data=True)):
        if i % 10000 == 0:
            print("Initializing node {}/{}".format(i, len(G.nodes)), end='\r')

        ## set distance counter
        D[node_id] = m.continuous_var(name=name(node_id, var='d'))
        m.add_constraint(ct=D[node_id] >= 0,
                         ctname="{} >= 0".format(name(source, target, var='d')))

    logger.info('Initialized variables')

    ######################
    # Objective function #
    ######################

    obj_func = 0

    # A), B) ~> suppose full-fiber for now
    for node_id, data in G.nodes(data=True):
        obj_func += D[node_id] * data['n_lines'] * params['c_f']

    # C)
    for source, target, data in G.edges(data=True):
        obj_func += X[source][target] * data['length'] * params['c_e']

    # D)
    for source, target in G.out_edges('r'):
        obj_func += X[source][target] * params['c_r']

    m.set_objective('min', obj_func)

    logger.info('Initialized objective function')

    ###############
    # Constraints #
    ###############

    for i, (node_id, data) in enumerate(G.nodes(data=True)):
        if i % 1000 == 0:
            print("Constraints on node {}/{}".format(i, len(G.nodes)), end='\r')

        in_count_X = 0
        in_count_N = 0
        for source, target in G.in_edges(node_id):
            in_count_X += X[source][target]
            in_count_N += N[source][target]

        out_count_X = 0
        out_count_N = 0
        for source, target in G.out_edges(node_id):
            out_count_X += X[source][target]
            out_count_N += N[source][target]

        # 2)
        if node_id == 'r':
            m.add_constraint(ct=in_count_X == 0,
                             ctname=name(node_id, var='in_count_X'))
            # terminal node
        elif data['n_lines'] > 0:
            m.add_constraint(ct=in_count_X == 1,
                             ctname=name(node_id, var='in_count_X'))
        else:
            m.add_constraint(ct=in_count_X <= 1,
                             ctname=name(node_id, var='in_count_X'))

        # 3)
        if node_id == 'r':
            m.add_constraint(ct=out_count_X >= 1,
                             ctname=name(node_id, var='out_count_X_lower'))

        # 4)
        m.add_constraint(ct=D[node_id] <= in_count_X * params['d_M'],
                         ctname="{}".format(name(node_id, var='distance_domain')))

        # 7), 8)
        if node_id != 'r':
            m.add_constraint(ct=in_count_N - out_count_N == data['n_lines'],
                             ctname="{}".format(name(node_id, var='flow_balance')))
        else:
            total_population = sum(data_['n_lines'] for _, data_ in G.nodes(data=True))
            m.add_constraint(ct=out_count_N == total_population,
                             ctname="{}".format(name(node_id, var='flow_balance')))

    logger.info('Set constraints 2, 3, 4, 7, 8')

    for i, (source, target, data) in enumerate(G.edges(data=True)):
        if i % 1000 == 0:
            print("Constraints on edge {}/{}".format(i, len(G.edges)), end='\r')

        edge_length = data['length']

        # 5)
        m.add_constraint(ct=D[target] - D[source] >= edge_length * X[source][target] - params['d_M'] * (1 - X[source][target]),
                         ctname="{}".format(name(source, target, var='distance_upper')))

        m.add_constraint(ct=D[target] - D[source] <= edge_length * X[source][target] + params['d_M'] * (1 - X[source][target]),
                         ctname="{}".format(name(source, target, var='distance_lower')))

        # 6)
        m.add_constraint(ct=N[source][target] <= params["n_M"] * X[source][target],
                         ctname="{}".format(name(source, target, var='n_max')))

    logger.info('Set constraints 5, 6')

    with open(logfile, "w") as f:
        solver = m.solve(log_output=f)

    if solver is None:
        logger.error("Unable to solve ILP")
        exit(1)
    else:
        # m.print_solution()
        pass

    # save results to graph
    for source, target in G.edges:
        G[source][target]['x'] = X[source][target].solution_value
        G[source][target]['n'] = N[source][target].solution_value

    for node_id in G.nodes:
        G.node[node_id]['d'] = D[node_id].solution_value

    # allow painless conversion to GraphML format
    for node_id, data in G.nodes(data=True):
        data_copy = data.copy()
        data.clear()

        data['d'] = float(data_copy['d'])
        data['lon'] = float(data_copy['lon'])
        data['lat'] = float(data_copy['lat'])

        # mark active and subroot nodes
        data['active'] = data_copy['n_lines'] > 0

        ## NOTE that n_lines change meaning, as current terminals will be ignored in further iterations in favour of current sub-roots

        # check subroots
        if G['r'][node_id]['x'] == 1:
            data['is_subroot'] = True
            data['n_lines'] = int(N['r'][node_id])
        else:
            data['is_subroot'] = False
            data['n_lines'] = 0

    # remove artificial root node
    G.remove_node('r')

    for _, _, data in G.edges(data=True):
        data_copy = data.copy()
        data.clear()

        data['length'] = float(data_copy['length'])
        data['x'] = int(data_copy['x'])
        data['n'] = int(data_copy['n'])

        # mark active edges
        data['active'] = data['x'] == 1

    return G
def convert_properties_nx(G, out_format, vp=['lon', 'lat'], ep=['length']):
    for _, data in G.nodes(data=True):
        for prop in vp:
            data[prop] = out_format(data[prop])

    for _, _, data in G.edges(data=True):
        for prop in ep:
            data[prop] = out_format(data[prop])

# load graph
G = nx.read_graphml(graph_path + "_complete.graphml")
convert_properties_nx(G, float)

# find optimal configuration
G_prime = ILP_solve(G, dict(params), "logs/DSLAM_ILP.log", 0.02)

# output to graphml file
convert_properties_nx(G_prime, str)
nx.write_graphml(G_prime, graph_path + "_DSLAM_ILP.graphml")
logger.info("Graph saved to file")
