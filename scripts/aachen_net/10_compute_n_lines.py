#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
closest_nodes_path="data/aachen_net/closest_nodes.csv"
graph_path="data/aachen_net/aachen_graph"
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
district_map = gpd.read_file("data/aachen_net/aachen_district_map.shp")
del district_map['FLäcHE'] # whole zero column

logger.info('districts ok')
def compute_distance(point1, point2, lon_label='lon', lat_label='lat'):
    g = Geodesic.WGS84.Inverse(point1[lat_label], point1[lon_label],
                               point2[lat_label], point2[lon_label])

    return g['s12']

def node_distance(G, source, target, **kwargs):
    point1 = G.node[source]
    point2 = G.node[target]

    return compute_distance(point1, point2, **kwargs)

def refresh_distances(G, **kwargs):
    for source, target in G.edges():
        G[source][target]['length'] = node_distance(G, source, target, **kwargs)
import graph_tool as gt
import networkx as nx

def get_prop_type(value, key=None):
    """
    Performs typing and value conversion for the graph_tool PropertyMap class.
    If a key is provided, it also ensures the key is in a format that can be
    used with the PropertyMap. Returns a tuple, (type name, value, key)
    """
    # Deal with the value
    if isinstance(value, bool):
        tname = 'bool'

    elif isinstance(value, int):
        tname = 'float'
        value = float(value)

    elif isinstance(value, float):
        tname = 'float'

    elif isinstance(value, dict):
        tname = 'object'

    else:
        tname = 'string'
        value = str(value)

    return tname, value, key


def nx2gt(nxG):
    """
    Converts a networkx graph to a graph-tool graph.
    """
    # Phase 0: Create a directed or undirected graph-tool Graph
    gtG = gt.Graph(directed=nxG.is_directed())

    # Add the Graph properties as "internal properties"
    for key, value in nxG.graph.items():
        # Convert the value and key into a type for graph-tool
        tname, value, key = get_prop_type(value, key)

        prop = gtG.new_graph_property(tname) # Create the PropertyMap
        gtG.graph_properties[key] = prop     # Set the PropertyMap
        gtG.graph_properties[key] = value    # Set the actual value

    # Phase 1: Add the vertex and edge property maps
    # Go through all nodes and edges and add seen properties
    # Add the node properties first
    nprops = set() # cache keys to only add properties once
    for node, data in nxG.nodes(data=True):

        # Go through all the properties if not seen and add them.
        for key, val in data.items():
            if key in nprops: continue # Skip properties already added

            # Convert the value and key into a type for graph-tool
            tname, _, key  = get_prop_type(val, key)

            prop = gtG.new_vertex_property(tname) # Create the PropertyMap
            gtG.vertex_properties[key] = prop     # Set the PropertyMap

            # Add the key to the already seen properties
            nprops.add(key)

    # Also add the node id: in NetworkX a node can be any hashable type, but
    # in graph-tool node are defined as indices. So we capture any strings
    # in a special PropertyMap called 'id' -- modify as needed!
    gtG.vertex_properties['id'] = gtG.new_vertex_property('string')

    # Add the edge properties second
    eprops = set() # cache keys to only add properties once
    for src, dst, data in nxG.edges(data=True):

        # Go through all the edge properties if not seen and add them.
        for key, val in data.items():
            if key in eprops: continue # Skip properties already added

            # Convert the value and key into a type for graph-tool
            tname, _, key = get_prop_type(val, key)

            prop = gtG.new_edge_property(tname) # Create the PropertyMap
            gtG.edge_properties[key] = prop     # Set the PropertyMap

            # Add the key to the already seen properties
            eprops.add(key)

    # Phase 2: Actually add all the nodes and vertices with their properties
    # Add the nodes
    vertices = {} # vertex mapping for tracking edges later
    for node, data in nxG.nodes(data=True):

        # Create the vertex and annotate for our edges later
        v = gtG.add_vertex()
        vertices[node] = v

        # Set the vertex properties, not forgetting the id property
        data['id'] = str(node)
        for key, value in data.items():
            gtG.vp[key][v] = value # vp is short for vertex_properties

    # Add the edges
    for src, dst, data in nxG.edges(data=True):

        # Look up the vertex structs from our vertices mapping and add edge.
        e = gtG.add_edge(vertices[src], vertices[dst])

        # Add the edge properties
        for key, value in data.items():
            gtG.ep[key][e] = value # ep is short for edge_properties

    # Done, finally!
    return gtG
def networkx_to_geopandas(G, lon_label='lon', lat_label='lat', projection=None):
    G_gt = nx2gt(G)
    return graph_tool_to_geopandas(G_gt, lon_label, lat_label, projection)

def graph_tool_to_geopandas(G, lon_label='lon', lat_label='lat', projection=None):
    ## collect nodes info
    nodes_df = gpd.GeoDataFrame()
    for key, property_map in G.vertex_properties.items():
        property_map = G.vertex_properties[key]
        nodes_df[key] = np.asarray(list(property_map),
                                   dtype=property_map.python_value_type())

    nodes_df['geometry'] = nodes_df.apply(
        lambda row: Point(row[lon_label], row[lat_label]), axis=1
    )
    nodes_df['gt_id'] = [int(v) for v in G.vertices()]

    nodes_df.crs = {'init' :'epsg:4326'} # long-lat projection
    if projection:
        nodes_df = nodes_df.to_crs(projection.srs)

    ## collect edges info
    edges_df = gpd.GeoDataFrame()
    for key, property_map in G.edge_properties.items():
        property_map = G.edge_properties[key]
        edges_df[key] = np.asarray(list(property_map),
                                   dtype=property_map.python_value_type())

    # save which source and target node_id
    if len(edges_df) > 0:
        edges_ids = ( (int(edge.source()), int(edge.target())) for edge in G.edges() )
        edges_df['gt_source'], edges_df['gt_target'] = list(zip(*edges_ids))

    # create line, using also node information
    def get_segment(G, edge, lon_label, lat_label):
        source_id = int(edge.source())
        target_id = int(edge.target())

        segment = LineString( ((G.vertex_properties['lon'][source_id],
                                G.vertex_properties['lat'][source_id]),
                               (G.vertex_properties['lon'][target_id],
                                G.vertex_properties['lat'][target_id])) )
        return segment

    edges_df['geometry'] = list(map(lambda x: get_segment(G, x, lon_label, lat_label), G.edges()))

    edges_df.crs = {'init' :'epsg:4326'} # long-lat projection
    if projection:
        edges_df = edges_df.to_crs(projection.srs)

    return nodes_df, edges_df

def graph_to_geopandas(G, lon_label='lon', lat_label='lat', projection=None):
    if isinstance(G, nx.DiGraph) or isinstance(G, nx.Graph):
        return networkx_to_geopandas(G, lon_label, lat_label, projection)

    if isinstance(G, gt.Graph):
        return graph_tool_to_geopandas(G, lon_label, lat_label, projection)

    raise ValueError("Unrecognized graph object {}".format(type(G)))
def plot_graph(G, lon_label='lon', lat_label='lat', ax=None, figsize=(6, 6), title=None, projection=None):
    nodes_df, edges_df = graph_to_geopandas(G, lon_label, lat_label, projection)
    plot_geopandas_graph(nodes_df, edges_df, ax, figsize, title)

def plot_geopandas_graph(nodes_df, edges_df, ax=None,
                                             figsize=(6, 6),
                                             title=None,
                                             projection=None,
                                             root_markersize=2,
                                             edges_linewidth=0.5):
    ## plot everything
    if ax is None:
        fig = plt.figure(figsize=figsize, frameon=False)
        ax = fig.gca()

    if title:
        ax.set_title(title + "\n",
                     fontsize=15,
                     fontweight=font_spec['font.weight'])

    nodes_df.plot(ax=ax,
                  markersize=1,
                  color='black',
                  zorder=2)

    if 'is_subroot' in nodes_df.columns:
        nodes_df[nodes_df['is_subroot']].plot(ax=ax,
                                              markersize=root_markersize,
                                              color='red',
                                              zorder=3)

    edges_df.plot(ax=ax,
                  color='black',
                  # column='weight',
                  linewidth=edges_linewidth,
                  zorder=1)

    plt.axis('off')
def convert_properties_nx(G, out_format, vp=['lon', 'lat'], ep=['length']):
    for _, data in G.nodes(data=True):
        for prop in vp:
            data[prop] = out_format(data[prop])

    for _, _, data in G.edges(data=True):
        for prop in ep:
            data[prop] = out_format(data[prop])
def convert_properties(G, out_format, vp=['lon', 'lat'], ep=['length']):
    if isinstance(G, nx.DiGraph) or isinstance(G, nx.Graph):
        convert_properties_nx(G, out_format, vp, ep)

    if out_format == str:
        gt_format = 'string'
    elif out_format == float:
        gt_format = 'double'
    else:
        raise ValueError("Invalid format")

    if isinstance(G, gt.Graph):
        for prop in vp:
            # create new map
            new_prop = G.new_vertex_property(gt_format)

            for v in G.vertices():
                new_prop[v] = out_format(G.vp[prop][v])

            del G.vp[prop]
            G.vp[prop] = new_prop

        for prop in ep:
            # create new map
            new_prop = G.new_edge_property(gt_format)

            for e in G.edges():
                new_prop[e] = out_format(G.ep[prop][e])

            del G.ep[prop]
            G.ep[prop] = new_prop
district_population = pd.read_csv("data/aachen_net/20170630_population_density.csv")
district_population.columns = ['STATBEZ', 'PERS']

# join using index
district_map.set_index('STATBEZ', inplace=True)
district_population.set_index('STATBEZ', inplace=True)

district_map['population'] = district_population['PERS']

# compute area in km^2: I checked some in wikipedia to be sure
district_map['area'] = district_map['geometry'].area / 10**6
district_map['density'] = district_map['population'] / district_map['area']

logger.info('population ok')

with open(graph_path + "_2_temp.json") as f:
    js_graph = json.load(f)

G = json_graph.node_link_graph(js_graph)

## assign the district by majority vote on area

# nodes with no area have not to be assigned any district
for node_id in G.nodes():
    node_data = G.node[node_id]

    node_data['area'] = 0
    node_data['district'] = None

    # override values if needed
    if 'district_count' in node_data:
        area_count = node_data['district_count']

        node_data['area'] = sum(area_count.values())
        node_data['district'] = max(area_count, key=lambda x: area_count[x])

        del node_data['district_count']

## split population across all nodes in the same district

# compute total building area per district
district_area_map = { id_: 0 for id_ in district_map.index }
for node_id, data in G.nodes(data=True):
    if data['district']:
        district_area_map[int(data['district'])] += data['area']

# distribute population accordingly
for node_id in G.nodes():
    node_data = G.node[node_id]

    if node_data['district']:
        district_id = int(node_data['district'])

        node_data['population'] = \
            node_data['area'] / \
            district_area_map[district_id] * \
            district_map.loc[district_id].population
    else:
        node_data['population'] = 0

    del node_data['area']
    del node_data['district']

## this section is /completely/ heuristic! done to match probable (supposed)
## numbers about Aachen network

# compute number of lines per node, given assigned population
for node_id in G.nodes():
    node_data = G.node[node_id]

    n_lines = int(node_data['population'] // 6)

    # avoid number of lines too small or too big
    if n_lines < 1:
        node_data['n_lines'] = 0
    elif n_lines > 48:
        node_data['n_lines'] = 48
    else:
        node_data['n_lines'] = n_lines

    del node_data['population']

with open(graph_path + "_2_added_buildings.json", 'w') as output:
    output.write(json.dumps(json_graph.node_link_data(G)))

# since this is the last process, just save it as "complete"
# allow painless conversion to GraphML format
for _, data in G.nodes(data=True):
    data_copy = data.copy()
    data.clear()

    data['lon'] = float(data_copy['lon'])
    data['lat'] = float(data_copy['lat'])
    data['n_lines'] = int(data_copy['n_lines'])

    # compatibility data for plot
    data['active'] = True
    data['is_subroot'] = False

for _, _, data in G.edges(data=True):
    data_copy = data.copy()
    data.clear()

    data['length'] = float(data_copy['length'])

    # compatibility data for plot
    data['active'] = True

convert_properties(G, str)
nx.write_graphml(G, graph_path + "_complete.graphml")
