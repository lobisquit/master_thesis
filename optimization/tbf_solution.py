import itertools
from random import choice, random, seed

import networkx as nx
import numpy as np
import pandas as pd
from scipy.optimize import LinearConstraint, minimize
from scipy.sparse import csr_matrix

from problem_def import *

cache = {}
def get_subtree_leaves(g, v):
    if v in cache:
        return cache[v]

    if g.in_degree(v) == 0:
        cache[v] = v
        return [v]
    else:
        child_leaves = []
        for child, _ in g.in_edges(v):
            child_leaves += get_subtree_leaves(g, child)

        cache[v] = child_leaves
        return child_leaves

def get_children(g, v):
    in_edges = g.in_edges(v)
    sources, targets = list(zip(*in_edges))

    return list(sources)

def build_validator(users, g, n_leaves):
    n_nodes = len(g.nodes())
    n_constraints = n_nodes - n_leaves

    # constraints matrix
    A = csr_matrix((n_constraints, n_nodes))
    b = np.zeros( (n_constraints, 1) )

    current_node_idx = 0

    mainframe = [node for node, attrs in g.nodes(data=True) if attrs['root']][0]

    # set mainframe constraint
    A[current_node_idx, get_subtree_leaves(g, mainframe)] = 1
    b[current_node_idx] = MAX_MAINFRAME_BW
    current_node_idx += 1

    for router in get_children(g, mainframe):

        for dslam in get_children(g, router):
            A[current_node_idx, get_subtree_leaves(g, dslam)] = 1
            b[current_node_idx] = MAX_DSLAM_BW
            current_node_idx += 1

        A[current_node_idx, get_subtree_leaves(g, router)] = 1
        b[current_node_idx] = MAX_ROUTER_BW
        current_node_idx += 1

    def is_valid(bws):
        test = A.dot(bws) - b
        return np.all(test < 0)

    return is_valid

def perturb(bws, users, max_perturb):
    user = choice(users)
    delta = random() * max_perturb

    # given problem specification, going forward is (almost always) a good
    # idea: never go back
    new_bws = bws.copy()
    new_bws[user] += delta

    return new_bws

p_nothing = 0.2
p_streaming = 0.6

g = nx.read_graphml('abstract_topology.graphml')

# fix loading problems
renamer = dict(zip(
    list(g.nodes()),
    [int(s[1:]) for s in g.nodes()]
))
g = nx.relabel_nodes(g, renamer)

n_nodes = len(g.nodes())
leaves = [node for node in g.nodes() if len(g.in_edges(node)) == 0]

users = []
bws_min    = np.zeros( (n_nodes,) )
tolerances = np.zeros( (n_nodes,) )
margins    = np.zeros( (n_nodes,) )

for leaf in leaves:
    bw_min, tolerance, margin = get_realization(p_nothing, p_streaming)

    if bw_min > 0:
        users.append(leaf)

        bws_min[leaf] = bw_min
        tolerances[leaf] = tolerance
        margins[leaf] = margin

users = np.array(users)
is_valid = build_validator(users, g, len(leaves))

# initial guess
bws = np.zeros( (n_nodes,) )
bws[users] = 1

# start with a nice perturbation
temperature = 1e5 # bit/s

TEMP_DROP = 0.9
TEMP_STEP = 1000
MAX_BLOCKED_ITERS = 10000

n_iter = 1
n_blocked_iters = 1
while True:
    new_bws = perturb(bws, users, temperature)

    if is_valid(bws):
        bws = new_bws
        n_blocked_iters = 1
    else:
        n_blocked_iters += 1

    n_iter += 1

    # change perturbation
    if n_blocked_iters % MAX_BLOCKED_ITERS == 0:
        print("EXIT: idle for {} iterations".format(n_blocked_iters))
        break

    print("Temperature", temperature, 'n_iter', n_iter)

    if n_iter % TEMP_STEP == 0:
        temperature *= TEMP_DROP

print("OBJ:", obj_function(bws, bws_min, tolerances, margins))
