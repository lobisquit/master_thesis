import itertools
import logging
from random import choice, random, seed

import networkx as nx
import numpy as np
import pandas as pd
from scipy.optimize import LinearConstraint, minimize
from scipy.sparse import csr_matrix, lil_matrix

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
    A = lil_matrix((n_constraints, n_nodes))
    b = np.zeros( (n_constraints, ) )

    current_node_idx = 0

    mainframe = [node for node, attrs in g.nodes(data=True) if attrs['root']][0]

    # set mainframe constraint
    A[current_node_idx, get_subtree_leaves(g, mainframe)] = 1
    b[current_node_idx] = 1e15
    current_node_idx += 1

    for router in get_children(g, mainframe):

        for dslam in get_children(g, router):
            A[current_node_idx, get_subtree_leaves(g, dslam)] = 1
            b[current_node_idx] = MAX_DSLAM_BW
            current_node_idx += 1

        A[current_node_idx, get_subtree_leaves(g, router)] = 1
        b[current_node_idx] = MAX_ROUTER_BW
        current_node_idx += 1

    A = csr_matrix(A)

    def is_valid(bws):
        test = A.dot(bws) - b
        return np.all(test <= 0)

    return is_valid

def dummy_validator(g, bws):
    mainframe = [node for node, attrs in g.nodes(data=True) if attrs['root']][0]

    mainframe_users = get_subtree_leaves(g, mainframe)
    assert bws[mainframe_users].sum() < MAX_MAINFRAME_BW

    for router in get_children(g, mainframe):
        router_users = get_subtree_leaves(g, router)
        assert bws[router_users].sum() < MAX_ROUTER_BW

        for dslam in get_children(g, router):
            dslam_users = get_subtree_leaves(g, dslam)
            assert bws[dslam_users].sum() < MAX_DSLAM_BW

    return True

def run_optimization(p_nothing, p_streaming, logger):
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
    bws_min = np.zeros( (n_nodes,) )
    a = np.zeros( (n_nodes,) )
    b = np.zeros( (n_nodes,) )

    for leaf in leaves:
        single_a, single_b, bw_min = get_realization(p_nothing, p_streaming)

        if single_a != 0:
            users.append(leaf)

            bws_min[leaf] = bw_min
            a[leaf] = single_a
            b[leaf] = single_b

    streaming_users = np.where(bws_min > 10)[0]

    users = np.array(users)
    n_users = len(users)
    is_valid = build_validator(users, g, len(leaves))

    # initial guess
    bws = np.zeros( (n_nodes,) )
    bws[users] = 500e3

    assert is_valid(bws), "Initial solution not valid"

    # start with a nice perturbation
    temperature = 500e3 # bit/s
    size = 10

    SIZE_DROP = 0.9
    TEMP_DROP = 0.99
    EPOCH = 10e3
    MAX_BLOCKED_ITERS = 10
    MAX_BLOCKED_ITERS_STEP = 1e-5

    old_obj = -np.inf
    n_iter = 1
    n_blocked_iters = 1
    while True:
        user = np.random.choice(streaming_users,
                                size=(np.ceil(size).astype(int),))

        delta = np.random.random(user.shape) * temperature

        # given problem specification, going forward is (almost always) a good
        # idea: never go back
        bws[user] += delta

        if is_valid(bws):
            n_blocked_iters = 1
        else:
            # revert change
            bws[user] -= delta
            n_blocked_iters += 1

        n_iter += 1

        # change perturbation
        if n_iter % EPOCH == 0:
            temperature *= TEMP_DROP
            size *= SIZE_DROP

        if n_iter % 10000 == 0:
            obj = obj_function(bws, a, b)
            logger.debug("OBJ: {}, p_nothing {}, p_streaming {}, T {}"\
                         .format(obj, p_nothing, p_streaming, temperature))

            # analyze improvement
            if abs(obj - old_obj) < MAX_BLOCKED_ITERS_STEP:
                n_blocked_iters += 1
            else:
                n_blocked_iters = 0

            # update previous obj function value
            old_obj = obj

            # stop if it has been negligible for too long
            if n_blocked_iters == MAX_BLOCKED_ITERS:
                logger.info("Negligible improvement in last rounds: declare convergence")
                break

    utilities = utility(bws, a, b)
    obj = obj_function(bws, a, b)
    logger.debug("RESULT: {}".format(obj))

    return obj, n_users, utilities

if __name__ == '__main__':
    logger = logging.getLogger('aachen_net.org')
    logger.setLevel(logging.DEBUG)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s::%(levelname)s::%(module)s::%(message)s",
                                  "%Y-%m-%d %H:%M:%S")

    ch = logging.StreamHandler()
    ch.setLevel(logging.DEBUG)
    ch.setFormatter(formatter)
    logger.addHandler(ch)

    N_SEEDS = 2
    N_NOTHING = 10
    N_STREAM = 3

    results = []
    for s in range(N_SEEDS):
        np.random.seed(s)
        seed(s)

        for j, p_streaming in enumerate(np.linspace(0.1, 0.9, N_STREAM)):
            for i, p_nothing in enumerate(np.linspace(0.1, 0.9, N_NOTHING)):
                logger.info("seed {}/{}, p_streaming {}/{}, p_nothing {}/{}"\
                            .format(s, N_SEEDS,
                                    j, N_STREAM,
                                    i, N_NOTHING))

                obj, n_users, _ = run_optimization(p_nothing, p_streaming, logger)
                results.append({
                    'obj': obj,
                    'n_users': n_users,
                    'p_nothing': p_nothing,
                    'p_streaming': p_streaming,
                    'seed': s
                })

    pd.DataFrame(results).to_csv(
        '../data/optimization/heuristic.csv',
        index=None
    )
