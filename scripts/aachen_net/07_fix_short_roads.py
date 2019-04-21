#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
graph_path="data/aachen_net/aachen_graph"
min_length="20"
import csv
import json
import logging
import math
from math import ceil, sqrt
from pathlib import Path

import fiona
import geopandas as gpd
import h5py
import matplotlib.pyplot as plt
import networkx as nx
import numpy as np
import pandas as pd
from geographiclib.geodesic import Geodesic
from graph_tool.all import *
from matplotlib import rcParams
from networkx.readwrite import json_graph
from pyproj import Proj
from s2g import ShapeGraph
from scipy import spatial
from scipy.spatial.distance import cdist
from shapely import wkt
from shapely.geometry import LineString, Point, shape
from shapely.geometry.polygon import Polygon
from shapely.ops import cascaded_union, nearest_points

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
font_spec = {
    'font.family':'sans-serif',
    'font.sans-serif':['Fira Sans'],
    'font.weight': 'regular'
}
rcParams.update(font_spec)

logger.info('matplotlib ok')

# load graph
with open(graph_path + "_0_raw.json") as f:
    js_graph = json.load(f)

G = json_graph.node_link_graph(js_graph)

assert nx.is_connected(G), "Raw G is not connected"

## remove too short roads

MIN_LENGTH = int(min_length)

def order_edge(edge):
    return min(edge), max(edge)

# precompute expensive distance dictionary (update each cycle)
edge_length_map = { order_edge(edge): node_distance(G, *edge)
                    for edge in G.edges() }

# proceed splitting all roads that are shorter than MIN_LENGTH
while True:
    current_min_length = float('inf')
    min_source = None
    min_target = None

    n = 0
    # compute length of each road
    for edge, length in edge_length_map.items():
        # keep track of the shortest road
        if length < current_min_length:
            current_min_length = length
            min_source, min_target = edge

        # count how many are still there
        if length < MIN_LENGTH:
            n += 1

    if current_min_length > MIN_LENGTH:
        break

    # segment from min_source to min_target
    min_g = Geodesic.WGS84.Inverse(
        G.node[min_source]['lat'], G.node[min_source]['lon'],
        G.node[min_target]['lat'], G.node[min_target]['lon']
    )

    # use mid-point for contracted node position
    mid_point = Geodesic.WGS84.Direct(lat1= G.node[min_source]['lat'],
                                      lon1= G.node[min_source]['lon'],
                                      azi1= min_g['azi1'],
                                      s12=  min_g['s12']/2)

    # new edges from min_target will be from min_source
    # work on (min_target, ...) but avoid (min_target, min_source)
    new_edges = [ (min_source, w)
                  for x, w in G.edges(min_target)
                  if w != min_source ]

    # remove edges touching min_target from the lengths dictionary
    for edge in G.edges(min_target):
        del edge_length_map[ order_edge(edge) ]

    # remove node and its edges and add new ones
    G.remove_node(min_target)
    G.add_edges_from(new_edges)

    # move node to keep in the middle point
    G.node[min_source].clear()
    G.node[min_source]['lat'] = mid_point['lat2']
    G.node[min_source]['lon'] = mid_point['lon2']

    # min_source has moved: recompute distances for each edge
    for edge in G.edges(min_source):
        edge_length_map[ order_edge(edge) ] = node_distance(G, *edge)

    # compute distances for each of the new edges
    for edge in new_edges:
        edge_length_map[ order_edge(edge) ] = node_distance(G, *edge)

    print('{} remaining'.format(n - 1), end="\r")

# check if operation was successful
refresh_distances(G)
assert min(data['length'] for _, _, data in G.edges(data=True)) >= MIN_LENGTH

assert nx.is_connected(G), "Intermediate G is not connected"

with open(graph_path + "_1_temp.json", 'w') as output:
    output.write(json.dumps(json_graph.node_link_data(G)))
