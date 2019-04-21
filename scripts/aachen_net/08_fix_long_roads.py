#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
graph_path="data/aachen_net/aachen_graph"
min_length="20"
max_length="200"
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
with open(graph_path + "_1_temp.json", "r") as f:
    js_graph = json.load(f)

G = json_graph.node_link_graph(js_graph)

assert nx.is_connected(G), "Raw G is not connected!"

## split roads that are too long

MIN_LENGTH = int(min_length)
MAX_LENGTH = int(max_length)

# collect edges (not to mess up with G iterator)
edges_to_split_distance = { edge: node_distance(G, *edge)
                            for edge in G.edges()
                            if node_distance(G, *edge) >= MAX_LENGTH }

progress = 1
for (source, target), distance in edges_to_split_distance.items():
    print("{}/{} roads splitted".format(progress, len(edges_to_split_distance)), end='\r')
    progress += 1

    G.remove_edge(source, target)

    # number of new segments
    n_segments = int(ceil(distance / MAX_LENGTH))

    # n + source + target now are in the segment
    delta = distance / n_segments

    if delta > MAX_LENGTH:
        print("Nope", delta)
        exit(1)

    # run along segment from source to target
    g = Geodesic.WGS84.Inverse(
        G.node[source]['lat'], G.node[source]['lon'],
        G.node[target]['lat'], G.node[target]['lon']
    )

    new_points = []
    for i in range(1, n_segments):
        # disseminate points every delta
        point = Geodesic.WGS84.Direct(lat1= G.node[source]['lat'],
                                      lon1= G.node[source]['lon'],
                                      azi1= g['azi1'],
                                      s12=  delta * i)

        new_points.append(max(G.nodes) + 1)
        G.add_node(max(G.nodes) + 1,
                   lat=point['lat2'],
                   lon=point['lon2'])

    G.add_edge(source, new_points[0])

    for j in range(n_segments - 2):
        G.add_edge(new_points[j], new_points[j+1])

    G.add_edge(new_points[-1], target)

# check distances respect the constraints
refresh_distances(G)
assert max(data['length'] for _, _, data in G.edges(data=True)) <= MAX_LENGTH, "Max length exceeded"
assert min(data['length'] for _, _, data in G.edges(data=True)) >= MIN_LENGTH, "Min length not respected"

assert nx.is_connected(G), "Processed G not connected!"

with open(graph_path + "_1_fix_roads.json", 'w') as output:
    output.write(json.dumps(json_graph.node_link_data(G)))
