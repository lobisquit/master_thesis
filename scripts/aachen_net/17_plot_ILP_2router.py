#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
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
prj_string_file = Path("data/aachen_net/aachen_district_map_prj.txt")
if not prj_string_file.is_file():
    import osr # troublesome to install in cluster

    prj_content = open('data/aachen_net/aachen_district_map.prj', 'r').read()
    srs = osr.SpatialReference()
    srs.ImportFromWkt(prj_content)

    with open(str(prj_string_file), 'w') as f:
        f.write(srs.ExportToProj4())

prj_string = open(str(prj_string_file), 'r').read()
projection = Proj(prj_string)

logger.info('projection ok')
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
def plot_result(G,
                projection=None,
                cut=True,
                title=None,
                ax=None,
                root_markersize=2,
                edges_linewidth=0.5):
    # convert to graph_tool if necessary
    if isinstance(G, nx.DiGraph) or isinstance(G, nx.Graph):
        G = nx2gt(G)

    # get geopandas elements to plot
    nodes_df, edges_df = graph_to_geopandas(G,
                                            projection=projection)

    if ax is None:
        fig = plt.figure(frameon=False)
        ax = fig.gca()

    plot_geopandas_graph(nodes_df[nodes_df['active']],
                         edges_df[edges_df['active']],
                         title=title,
                         ax=ax,
                         root_markersize=root_markersize,
                         edges_linewidth=edges_linewidth)

    edges_df.plot(color='#b2b2b2', zorder=-1, ax=ax, linewidth=edges_linewidth)

    if cut:
        ax.axis('off')
        ax.set_xlim(293117, 295351)
        ax.set_ylim(5627800, 5629570)
        plt.subplots_adjust(top=1.0,
                            bottom=0.0,
                            left=0.015,
                            right=0.977,
                            hspace=0.0,
                            wspace=0.0)

G = load_graph(graph_path + "_2router_ILP.graphml")
convert_properties(G, float)

plot_result(G,
            projection=projection)

# plt.show()
plt.savefig('figures/ILP_2router.png', dpi=250)
plt.close('all')
