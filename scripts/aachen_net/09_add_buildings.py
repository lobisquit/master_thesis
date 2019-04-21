#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
valid_types=["house", "residential", "apartments", "industrial", "school", "farm", "retail", "allotment_house", "warehouse", "office", "public", "civic", "hospital", "university", "manufacture", "dormitory", "community_centre", "hotel", "bungalow", "family_house", "commercial"]
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
district_map = gpd.read_file("data/aachen_net/aachen_district_map.shp")
del district_map['FLäcHE'] # whole zero column

logger.info('districts ok')
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
roads_path = "data/aachen_net/aachen_roads.shp"
roads_map = gpd.read_file(roads_path)
roads_map.OSM_ID = pd.to_numeric(roads_map.OSM_ID)
roads_map.crs = {'init': 'epsg:4326'}
roads_map = roads_map.to_crs(projection.srs)

logger.info('roads ok')
buildings_path = "data/aachen_net/aachen_buildings.shp"
buildings_map = gpd.read_file(buildings_path)
buildings_map.OSM_ID = pd.to_numeric(buildings_map.OSM_ID)
buildings_map.crs = {'init': 'epsg:4326'}
buildings_map = buildings_map.to_crs(projection.srs)

# set a custom label instead of None
buildings_map.loc[buildings_map['TYPE'].isnull(), 'TYPE'] = "UNSET"

# remove unwanted types, but keep UNSET ones
buildings_map = buildings_map[buildings_map['TYPE'].isin(valid_types + ['UNSET'])]

logger.info('buildings ok')

# load graph
with open(graph_path + "_1_fix_roads.json") as f:
    js_graph = json.load(f)

G = json_graph.node_link_graph(js_graph)

assert nx.is_connected(G), "Fixed roads G is not connected"

## filter out buildings, heuristically

# remove buildings that are too big or too small to be residential
buildings_map = buildings_map[ (buildings_map.area > 40) &
                               (buildings_map.area < 2000) ]

## assign each building area and district to a node

# pre-compute all (projected) node points with scipy.spatial.KDTree
node_ids = list(G.nodes)
node_coords = list(projection(data['lon'], data['lat'])
                   for _, data in G.nodes(data=True))
search_tree = spatial.KDTree(node_coords)

building_distances = []

building_index = 0
for _, building in buildings_map.iterrows():
    if building_index % 200 == 0:
        print("{}/{}".format(building_index, len(buildings_map)), end='\r')
    building_index += 1

    ## work only if building can be assigned to a district (not ones outside
    ## city)

    district_index = -1
    for index, district_row in district_map.iterrows():
        if building.geometry.centroid.within(district_row.geometry):
            district_index = index

    # avoid adding buildings which center is outside the city
    if district_index == -1:
        continue

    ## find closest point in the graph

    _, min_node_index = search_tree.query( (building.geometry.centroid.x,
                                            building.geometry.centroid.y) )
    node_id = node_ids[min_node_index]

    # measure building -> node distance precisely
    building_lon, building_lat = projection(building.geometry.centroid.x,
                                            building.geometry.centroid.y,
                                            inverse=True)

    node_lon, node_lat = projection(*node_coords[min_node_index], inverse=True)

    min_dist = compute_distance({'lon': building_lon, 'lat': building_lat},
                                {'lon': node_lon,     'lat': node_lat    })

    ## register value both in error measurer and graph

    building_distances.append(min_dist)

    node_data = G.node[node_id]

    # fill the structures if needed
    if 'district_count' not in node_data:
        node_data['district_count'] = {}

    if district_index not in node_data['district_count']:
        node_data['district_count'][district_index] = 0

    # update values for the node: each building contributes with its area to
    # the district count, in order to have

    # 1) total building area assigned to the node
    # 2) voting on district (based on area) to assigned the node to a district
    node_data['district_count'][district_index] += building.geometry.area

with open('data/aachen_net/buildings_position_error.csv', 'w') as f:
    for dist in building_distances:
        f.write("{}\n".format(dist))

with open(graph_path + "_2_temp.json", 'w') as output:
    output.write(json.dumps(json_graph.node_link_data(G)))
