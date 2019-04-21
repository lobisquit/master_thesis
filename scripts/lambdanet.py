labels_positions=[["Kiel", "center", "bottom"], ["Rostock", "center", "bottom"], ["Hamburg", "right", "bottom"], ["Berlin", "left", "center"], ["Dresden", "center", "top"], ["Leipzig", "center", "bottom"], ["Erfurt", "right", "center"], ["Coburg", "left", "center"], ["Nuremberg", "left", "center"], ["Ingolstadt", "left", "center"], ["Munich", "left", "center"], ["Karlsruhe", "right", "center"], ["Mannheim", "right", "center"], ["Stuttgart", "right", "top"], ["Augsburg", "right", "top"], ["Frankfurt", "left", "bottom"], ["Wurzburg", "center", "top"], ["Bremen", "right", "center"], ["Hannover", "right", "center"], ["Brunswick", "left", "bottom"], ["Hertford", "right", "center"], ["Gutersloh", "left", "top"], ["Dortmund", "left", "top"], ["Essen", "right", "bottom"], ["Dusseldorf", "right", "center"], ["Magdeburg", "center", "top"], ["Cologne", "left", "bottom"], ["Bonn", "right", "top"], ["Aachen", "right", "top"]]
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

path = "data/lambdanet/lambdanet.graphml"

G = nx.read_graphml(path)

## fix missing positions
# NOTE lots of them are removed because I don't need them (for now)

# this one is a guess (label is "None")
G.node['11']['Latitude'] = 53.384176
G.node['11']['Longitude'] = 11.766859
G.node['11']['label'] = ""

# Prague
G.node['9']['Latitude'] = 50.08804
G.node['9']['Longitude'] = 14.42076
G.remove_node('9')

# Stockholm
G.node['10']['Latitude'] = 59.334591
G.node['10']['Longitude'] = 18.063240
G.remove_node('10')

# Brno
G.node['17']['Latitude'] = 49.19522
G.node['17']['Longitude'] = 16.60796
G.remove_node('17')

# Vienna
G.node['18']['Latitude'] = 48.20849
G.node['18']['Longitude'] = 16.37208
G.remove_node('18')

# Bratislava
G.node['19']['Latitude'] = 48.14816
G.node['19']['Longitude'] = 17.10674
G.remove_node('19')

# London
G.node['23']['Latitude'] = 51.509865
G.node['23']['Longitude'] = -0.118092
G.remove_node('23')

# Zurich
G.node['28']['Latitude'] = 47.36667
G.node['28']['Longitude'] = 8.55
G.remove_node('28')

# Copenhagen
G.node['33']['Latitude'] = 55.67594
G.node['33']['Longitude'] = 12.56553
G.remove_node('33')

# Paris and Amsterdam (why are they in Germany?)
G.remove_node('20')
G.remove_node('34')

# fix Hannover spelling
G.node['40']['label'] = 'Hannover'

## collect data into proper lists
nodes = G.nodes(data=True)

nodes_info = []
for id_, data in nodes:
    point = Point(data['Longitude'], data['Latitude'])
    nodes_info.append({'geometry': point, **data})

edges_info = []
for node_id1, node_id2, data in G.edges(data=True):
    edge = LineString((
        (nodes[node_id1]['Longitude'], nodes[node_id1]['Latitude']),
        (nodes[node_id2]['Longitude'], nodes[node_id2]['Latitude'])
    ))

    edges_info.append({'geometry': edge, **data})

## provide GeoDataFrames
nodes_df = gpd.GeoDataFrame(nodes_info)
nodes_df.crs = {'init' :'epsg:4326'} # long-lat projection
nodes_df = nodes_df.to_crs(projection.srs)

edges_df = gpd.GeoDataFrame(edges_info)
edges_df.crs = {'init' :'epsg:4326'} # long-lat projection
edges_df = edges_df.to_crs(projection.srs)

# use geographical map of germany, as reference
states = gpd.read_file('data/lambdanet/germany_states.shp')
states.crs = {'init' :'epsg:4326'} # long-lat projection
states = states.to_crs(projection.srs)

## plot everything
fig = plt.figure(figsize=(4, 5), frameon=False)
ax = fig.gca()

ttl = ax.set_title("Map of LambdaNet",
                   fontsize=12,
                   fontweight=font_spec['font.weight'])

nodes_df.plot(ax=ax,
              markersize=1,
              color='black',
              zorder=2)

# draw name of wanted cities
# position of label has to be set by hand: damn
label_details = pd.DataFrame(labels_positions).set_index(0)
for _, x in nodes_df.iterrows():
   if x.label in label_details.index:
       ax.annotate(s=x.label,
                   xy=x.geometry.centroid.coords[0],
                   ha=label_details.loc[x.label][1],
                   va=label_details.loc[x.label][2],
                   fontsize=8,
                   zorder=5)

edges_df.plot(ax=ax,
              color='black',
              # column='weight',
              linewidth=0.5,
              zorder=1)

states.plot(facecolor='#ededed',
            linewidth=0.3,
            edgecolor='grey',
            zorder=0,
            ax=ax)

# highlight Aachen
aachen_point = nodes_df[nodes_df.label=="Aachen"]
aachen_point.plot(ax=ax,
                  color='red',
                  markersize=30,
                  edgecolor='black',
                  zorder=4)

plt.axis('off')
plt.tight_layout(rect=[-0.1, -0.1, 1, 1])
# plt.show()

print('If the script crashes, try using ipython')

plt.savefig('figures/german_backbone.pdf')
plt.close('all')
