#!/usr/bin/python3 
# -*- coding: utf-8 -*-
from __future__ import print_function
valid_types=["house", "residential", "apartments", "industrial", "school", "farm", "retail", "allotment_house", "warehouse", "office", "public", "civic", "hospital", "university", "manufacture", "dormitory", "community_centre", "hotel", "bungalow", "family_house", "commercial"]
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

fig = plt.figure(figsize=(6, 6), frameon=False)
ax = fig.gca()

district_map.plot(column='density',
                  cmap='viridis',
                  legend=True,
                  linewidth=0, # remove districts borders
                  alpha=0.7,
                  ax=ax)

roads_map.plot(color='black',
               alpha=0.6,
               linewidth=0.15,
               ax=ax)

buildings_map.plot(color='black',
                   alpha=0.5,
                   linewidth=0.15,
                   ax=ax)

# set title of colorbar (dirty trick)
fig.get_axes()[1].set_title('    people/km²',
                            fontweight=font_spec['font.weight'],
                            fontsize=12)

fig.get_axes()[1].tick_params(labelsize=10)

plt.axis('off')
plt.tight_layout(rect=[-1.25, -0.04, 1.2, 1])

# plt.show()

plt.savefig('figures/aachen_citymap.png', dpi=250)
plt.close('all')
