#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
params=[["n_M", 50], ["d_M", 1500], ["c_r", 31000], ["c_f", 3], ["c_e", 100]]
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

# send log also to file
fh = logging.FileHandler('logs/DSLAM_heuristic.log', mode='w')
fh.setLevel(logging.INFO)
fh.setFormatter(formatter)
logger.addHandler(fh)

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
def get_vertex_distance(G, cache_path=None, clear_cache=False):
    if cache_path is None:
        cache_path = Path('data/aachen_net/distance_matrix_temp.h5')

    # this takes some time: use cache file
    if not cache_path.exists() or clear_cache:
        vertex_dist = shortest_distance(G, weights=G.edge_properties['length'])

        with h5py.File(cache_path, 'w') as f:
            # obtain which and how many nodes have to be considered terminals
            N = G.num_vertices()

            # fill matrix with their pairwise distances
            matrix = f.create_dataset('vertex_dist', (N, N))
            for index, vertex_id in enumerate(G.vertices()):
                if index % 100 == 0:
                    print("Filling distance matrix", index, "/", N, end='\r')

                matrix[index, :] = np.array(vertex_dist[vertex_id])

    with h5py.File(cache_path, 'r') as f:
        # keep track of distance among vertices
        vertex_dist = np.array(f['vertex_dist'])

    # check simmetry
    assert np.allclose(vertex_dist, vertex_dist.T, equal_nan=True), \
        "Distance matrix not symmetric"

    return vertex_dist
paths = {}

def _order(i, j):
    return tuple(sorted([i, j]))

def compute_path(G, i, j, enable_cache=False):
    global paths

    if enable_cache:
        if _order(i, j) in paths:
            return paths[_order(i, j)]

    sp_edges = shortest_path(G, i, j)[1]
    ids = [G.edge_index[e] for e in sp_edges]

    # mark edges of the path and store the total path length
    mask = np.zeros(G.num_edges(), dtype=bool)
    mask[ids] = True

    edge_count = np.zeros(G.num_edges(), dtype=np.int)
    mask[ids] = 1

    if enable_cache:
        paths[_order(i, j)] = (mask, edge_count)

    return (mask, edge_count)

def get_min_path_tree(G, vertex_dist, cluster_nodes, is_terminal, params, fast_mode=False, enable_path_cache=True):
    # compute pairwise path for each couple of nodes in the cluster
    # and remove duplicate edges
    best_road_length = float('inf')
    best_root = None
    best_tree_edges = None

    # this tracks cable length in the best configuration *for the roads*:
    # choices had to be made to make computation fast
    best_cable_length = float('inf')

    # since root can be any node, explore non_terminal nodes also as candidate
    # in a neighbourhood. Top do so, look at the rows of vertex_dist
    # corresponding to the cluster, then pick all nodes (column indices) that
    # are close enough
    if fast_mode:
        neighbour_nodes = cluster_nodes
    else:
        _, neighbour_nodes = np.where(
            vertex_dist[cluster_nodes, :] <= params['discovery_dist']
        )

    for index, root in enumerate(set(neighbour_nodes)):
        # print("root {}/{}".format(index, len(set(neighbour_nodes))), end="\r")

        # compute edges of tree with root node
        tree_edges = np.zeros(G.num_edges(), dtype=bool)
        cable_edge_counts = np.zeros(G.num_edges(), dtype=np.int)

        current_cable_length = 0

        # tree from root has to reach all terminals
        for node in cluster_nodes:
            path_edges, edge_count = compute_path(G,
                                                  G.vertex(root),
                                                  G.vertex(node),
                                                  enable_cache=enable_path_cache)

            # a cable has to be placed per each customer
            cable_edge_counts += G.vp['n_lines'][G.vertex(node)] * edge_count
            tree_edges = np.logical_or(tree_edges, path_edges)

        current_road_lengths = G.ep['length'].a * tree_edges
        current_road_length = np.sum(current_road_lengths)

        if np.all(current_road_lengths) < params['d_M'] and \
           current_road_length < best_road_length:
            best_road_length  = current_road_length
            best_cable_length = np.sum(cable_edge_counts)
            best_root         = root
            best_tree_edges   = tree_edges

    if best_root is None:
        print("Error, root is None")
        exit(1)

    return best_road_length, best_cable_length, best_root, best_tree_edges
def update_metric(clusters, X, i, j, func=np.nanmax):
    i_idx = np.where(clusters == clusters[i])[0]
    j_idx = np.where(clusters == clusters[j])[0]

    X[i_idx, :] = func( (X[i, :], X[j, :]), axis=0 )
    X[j_idx, :] = X[i, :]

    X[:, i_idx] = X[i_idx, :].T
    X[:, j_idx] = X[j_idx, :].T

def disable_link(clusters, X, i, j):
    i_idx = np.where(clusters == clusters[i])[0]
    j_idx = np.where(clusters == clusters[j])[0]

    cmn_idx = np.hstack((i_idx, j_idx))
    X[np.ix_(cmn_idx, cmn_idx)] = np.nan

def objective_function(G, vertex_dist, clusters, is_terminal, params):
    # sub-root cost
    sub_root_cost = len(set(clusters)) * params['c_r']

    vertices = np.array([int(v) for v in G.vertices()])
    terminals = vertices[is_terminal]

    # use minimum path tree also for excavation (remove duplicate edges)
    total_cable_length = 0
    total_road_length = 0
    for index, cluster_id in enumerate(set(clusters)):
        cluster_nodes = terminals[clusters == cluster_id]

        # deal with simplest cases, avoiding MST computation
        if len(cluster_nodes) == 1:
            continue

        road_length, cable_length, _, _  = get_min_path_tree(G,
                                                             vertex_dist,
                                                             cluster_nodes,
                                                             is_terminal,
                                                             params,
                                                             fast_mode=True,
                                                             enable_path_cache=True)

        total_road_length += road_length
        total_cable_length += cable_length

    excavation_cost = total_road_length  * params['c_e']
    cable_cost      = total_cable_length * params['c_f']

    return sub_root_cost + cable_cost + excavation_cost

def heuristic_solve(G, params):
    global paths

    if isinstance(G, nx.DiGraph) or isinstance(G, nx.Graph):
        G = nx2gt(G)

    vertices = np.array([int(v) for v in G.vertices()])
    N = G.num_vertices()

    ## save information about terminals
    n_lines = np.array([G.vp['n_lines'][v] for v in G.vertices()], dtype=np.int)
    is_terminal = n_lines > 0

    terminals = vertices[is_terminal]
    terminal_lines = n_lines[is_terminal]

    T = len(terminals)

    # initialize clusters: at the beginning they are singletons of terminals
    clusters = terminals.copy()
    cluster_lines = terminal_lines.copy()

    vertex_dist = get_vertex_distance(G)
    terminal_dist = vertex_dist[np.ix_(is_terminal, is_terminal)]
    min_cluster_dist = terminal_dist.copy()

    initial_total_cluster_lines = sum(cluster_lines)

    # fill diagonal with nans (values we want to ignore)
    np.fill_diagonal(min_cluster_dist, np.nan)

    # create a matrix for the max distance between nodes in two different clusters
    # bij = diameter of cluster obtained joining i-th and j-th ones
    max_cluster_dist = min_cluster_dist.copy()

    # # prune the nodes that even at the beginning are further from each other more
    # # than the critical length
    min_cluster_dist[min_cluster_dist > params['d_M']] = np.nan
    max_cluster_dist[min_cluster_dist > params['d_M']] = np.nan

    total_couples = np.count_nonzero( np.isfinite(min_cluster_dist)) / 2
    logger.info("Pruning: left {:.0f} out of {}".format(total_couples, T * (T-1) // 2))

    # counters
    n_iter = 0
    previous_total_cost = float('inf')
    best_clusters = None

    while True:
        n_iter += 1

        # stop if all couples have been checked
        if np.all(np.isnan(min_cluster_dist)):
            logger.info("Checked all possible couples")
            break

        if n_iter % 50 == 0:
            total_cost = objective_function(G,
                                            vertex_dist,
                                            clusters,
                                            is_terminal,
                                            params)

            logger.info("Money {:.2f}M€ n_cluster {} cache_size {}".format(total_cost/1e6,
                                                                           len(set(clusters)),
                                                                           len(paths)))

            # stop if the found solution is no better than the previous one
            if total_cost > previous_total_cost:
                logger.info("Minimum exceeded")
                clusters = best_clusters
                break

            else:
                # update best solution with current one
                best_clusters = clusters.copy()
                previous_total_cost = total_cost

        # get two closest clusters (heuristic measure)
        min_rows, min_cols = np.where(min_cluster_dist == np.nanmin(min_cluster_dist))

        # i, j are the i-th, j-th cluster
        i, j = min_rows[0], min_cols[0]

        # ensure number of lines and diameter are not exceeded
        total_n_lines = cluster_lines[i] + cluster_lines[j]
        joint_diameter = max_cluster_dist[i][j]

        if total_n_lines <= params['n_M'] and \
           joint_diameter <= 2 * params['d_M']:
            # update merging conditions
            cluster_lines[i] = total_n_lines
            cluster_lines[j] = total_n_lines

            update_metric(clusters, min_cluster_dist, i, j, np.nanmin)
            update_metric(clusters, max_cluster_dist, i, j, np.nanmax)

            # set the same label for the two clusters (minimum given np.where convention)
            new_idx = min(clusters[i], clusters[j])

            clusters[clusters == clusters[i]] = new_idx
            clusters[clusters == clusters[j]] = new_idx

        # since it has been evaluated, remove couple (i, j) from the possibilities:
        # setting their distance to nan
        disable_link(clusters, min_cluster_dist, i, j)

    # save results in the graph
    # create maps if needed
    G.vp['is_subroot'] = G.new_vertex_property("bool")
    G.vp['active']     = G.new_vertex_property("bool")
    G.vp['father_id']  = G.new_vertex_property("int")
    G.ep['active']     = G.new_edge_property("bool")

    G.vp['is_subroot'].a = False
    G.vp['active'].a     = False
    G.vp['father_id'].a  = -1
    G.ep['active'].a     = False

    for index, cluster_id in enumerate(set(clusters)):
        if index % 100 == 0:
            print("Loaded cluster", index, "/", len(set(clusters)), end='\r')

        cluster_nodes = terminals[clusters == cluster_id]
        _, _, root, tree_edges = get_min_path_tree(G,
                                                   vertex_dist,
                                                   cluster_nodes,
                                                   is_terminal, params,
                                                   fast_mode=False,
                                                   enable_path_cache=False)

        # root may also be (in non-fast mode) a non-terminal, so its n_lines
        # would be zero: better set it
        G.vp['n_lines'].a[root] = cluster_lines[ np.where(clusters == cluster_id)[0][0] ]
        G.vp['is_subroot'].a[root] = True
        G.vp['active'].a[cluster_nodes] = True
        G.vp['father_id'].a[cluster_nodes] = root
        G.ep['active'].a = np.logical_or(G.ep['active'].a, tree_edges)

    # make n_lines != only for subroots (terminals of next iteration)
    not_subroot_mask = np.logical_not(G.vp['is_subroot'].a)
    G.vp['n_lines'].a[not_subroot_mask] = 0

    assert initial_total_cluster_lines == G.vp['n_lines'].a.sum(), 'Some lines were lost while merging'

    return G

np.warnings.filterwarnings('ignore')

# load graph
G = load_graph(graph_path + "_complete.graphml")
convert_properties(G, float)

for key, value in params:
    logger.info("{}={}".format(key, value))

# find optimal configuration
params = dict(params)
params['discovery_dist'] = 200
G_prime = heuristic_solve(G, params)

# output to graphml file
convert_properties(G_prime, str)
G_prime.save(graph_path + "_DSLAM_heuristic.graphml")
