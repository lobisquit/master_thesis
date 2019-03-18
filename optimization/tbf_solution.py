import itertools
from random import seed

import numpy as np
import pandas as pd
from graph_tool.all import *
from scipy.optimize import LinearConstraint, minimize

from problem_def import *

p_nothing = 0.2
p_streaming = 0.6

def equality_bw(father, children, n_nodes):
    A = np.zeros( (n_nodes, 1) )

    A[father] = -1
    A[children] = 1

    limit = np.zeros( (n_nodes, 1) )
    return LinearConstraint(A, limit, limit)

g = load_graph('../data/aachen_net/abstract_topology.graphml')

leaves = [int(v) for v in g.vertices() if g.vertex(v).in_degree() == 0]

# setup problem variables
bws_min    = np.zeros( (g.num_vertices(), ))
tolerances = np.zeros( (g.num_vertices(), ))
margins    = np.zeros( (g.num_vertices(), ))

for leaf in leaves:
    bw_min, tolerance, margin = get_realization(p_nothing, p_streaming)

    bws_min[leaf] = bw_min
    tolerances[leaf] = tolerance
    margins[leaf] = margin

active_user = (bws_min > 0)

# flow condition for bandwidth
constraints = []
mainframe = np.where(g.vp['root'].a)[0][0]
routers = g.get_in_edges(mainframe)[:, 0]

# constraints function shall return 0 when condition is met

constraints += [ equality_bw(mainframe, routers, g.num_vertices()) ]

for router in routers:
    dslams = g.get_in_edges(router)[:, 0]

    constraints += [ equality_bw(router, dslams, g.num_vertices()) ]

    for dslam in dslams:
        dslam_leaves = g.get_in_edges(dslam)[:, 0]
        users = [leaf for leaf in dslam_leaves if active_user[leaf]]

        # set flow condition only on active users
        constraints += [ equality_bw(dslam, users, g.num_vertices()) ]

def reverse_obj(x):
    return obj_function(-np.reshape(x, (-1, 1)),
                        np.reshape(bws_min, (-1, 1)),
                        np.reshape(tolerances, (-1, 1)),
                        np.reshape(margins, (-1, 1)))


x0 = np.ones((1, len(bws_min))) * 1e-9
result = minimize(reverse_obj, x0, constraints=constraints)
print(result)
