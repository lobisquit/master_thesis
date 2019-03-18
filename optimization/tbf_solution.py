import itertools
from random import seed

import networkx as nx
import numpy as np
import pandas as pd
from scipy.optimize import LinearConstraint, minimize

from problem_def import *

p_nothing = 0.2
p_streaming = 0.6

def equality_bw(father, children, n_nodes):
    A = np.zeros( (n_nodes, ) )

    A[father] = -1
    A[children] = 1

    return LinearConstraint(np.reshape(A, (1, -1)), 0, 0)

g = nx.read_graphml('abstract_topology.graphml')

# fix loading problems
renamer = dict(zip(
    list(g.nodes()),
    [int(s[1:]) for s in g.nodes()]
))
g = nx.relabel_nodes(g, renamer)

leaves = [node for node in g.nodes() if len(g.in_edges(node)) == 0]

# setup problem variables
bws_min    = np.zeros( (len(g.nodes()), ))
tolerances = np.zeros( (len(g.nodes()), ))
margins    = np.zeros( (len(g.nodes()), ))

for leaf in leaves:
    bw_min, tolerance, margin = get_realization(p_nothing, p_streaming)

    bws_min[leaf] = bw_min
    tolerances[leaf] = tolerance
    margins[leaf] = margin

active_user = (bws_min > 0)

# flow condition for bandwidth
constraints = []
mainframe = [node for node, attrs in g.nodes(data=True) if attrs['root']][0]
routers = list(list(zip(*g.in_edges(mainframe)))[0])

# constraints function shall return 0 when condition is met

constraints += [ equality_bw(mainframe, routers, len(g.nodes())) ]

for router in routers:
    dslams = list(list(zip(*g.in_edges(router)))[0])

    constraints += [ equality_bw(router, dslams, len(g.nodes())) ]

    for dslam in dslams:
        dslam_leaves = list(list(zip(*g.in_edges(dslam)))[0])
        users = [leaf for leaf in dslam_leaves if active_user[leaf]]

        # set flow condition only on active users
        constraints += [ equality_bw(dslam, users, len(g.nodes())) ]

def reverse_obj(x):
    return obj_function(-x,
                        bws_min,
                        tolerances,
                        margins)

x0 = np.ones( (len(bws_min), 1) ) * 1e-9
result = minimize(reverse_obj, x0, constraints=constraints)
print(result)
