import itertools
import logging
from random import choice, random, seed

import networkx as nx
import numpy as np
import pandas as pd
from scipy.optimize import LinearConstraint, minimize
from scipy.sparse import csr_matrix, lil_matrix

from problem_def import *

logger = logging.getLogger('aachen_net.org')
logger.setLevel(logging.DEBUG)
logger.propagate = False

formatter = logging.Formatter("%(asctime)s::%(levelname)s::%(module)s::%(message)s",
                              "%Y-%m-%d %H:%M:%S")

ch = logging.StreamHandler()
ch.setLevel(logging.DEBUG)
ch.setFormatter(formatter)
logger.addHandler(ch)

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

def run_optimization(p_nothing, p_streaming):
    g = nx.read_graphml('abstract_topology.graphml')

    # fix loading problems
    renamer = dict(zip(
        list(g.nodes()),
        [int(s[1:]) for s in g.nodes()]
    ))
    g = nx.relabel_nodes(g, renamer)

    n_nodes = len(g.nodes())
    leaves = [ node
               for node in g.nodes()
               if len(g.in_edges(node)) == 0]

    users = np.zeros((n_nodes,), dtype=bool)
    bws_min = np.zeros( (n_nodes,) )
    a = np.zeros( (n_nodes,) )
    b = np.zeros( (n_nodes,) )

    for leaf in leaves:
        single_a, single_b, bw_min = get_realization(p_nothing, p_streaming)

        if single_a != 0:
            users[leaf] = True

            bws_min[leaf] = bw_min
            a[leaf] = single_a
            b[leaf] = single_b

    users = np.array(users)
    n_users = users.sum()
    is_valid = build_validator(users, g, len(leaves))

    # initial guess
    bws = np.zeros( (n_nodes,) )
    bws[users] = 500e3

    assert is_valid(bws), "Initial solution not valid"

    # start with a nice perturbation
    temperature = 500e3 # bit/s
    size = 30

    SIZE_DROP = 0.9
    TEMP_DROP = 0.99
    EPOCH = 10e3
    MAX_BLOCKED_ITERS = 6e4
    MAX_BLOCKED_ITERS_STEP = 1e-4

    old_obj = -np.inf
    n_iter = 1
    n_blocked_iters = 1

    is_to_probe = np.ones((n_nodes, ), dtype=bool)

    # precumpute single utilities
    utilities = utility(bws, a, b)

    while True:
        active_users = users # np.logical_or(users, is_to_probe)

        # pick user according to worse utility
        user = np.random.choice(
            a=np.where(active_users)[0],
            size=( np.ceil(size).astype(int), ),
            # utility is always negative
            p=utilities[active_users] / utilities[active_users].sum()
        )

        delta = np.random.random(user.shape) * temperature

        # given problem specification, going forward is (almost always) a good
        # idea: never go back
        bws[user] += delta

        if is_valid(bws):
            n_blocked_iters = 1

            # update corresponding utility
            utilities[user] = utility(bws[user], a[user], b[user])
        else:
            # revert change
            bws[user] -= delta
            n_blocked_iters += 1
            is_to_probe[user] = False

        n_iter += 1

        # change perturbation
        if n_iter % EPOCH == 0:
            temperature *= TEMP_DROP
            size *= SIZE_DROP

        obj = np.log(utilities[users]).mean()

        # just checks
        assert np.all(utilities[users] <= 1), "Wrong utilities"
        assert obj == obj_function(bws, a, b), "Objective function does not correspond"

        # analyze improvement
        if abs(obj - old_obj) < MAX_BLOCKED_ITERS_STEP:
            n_blocked_iters += 1
        else:
            n_blocked_iters = 0

        # update previous obj function value
        old_obj = obj

        # stop if it has been negligible for too long
        if n_blocked_iters > MAX_BLOCKED_ITERS:
            logger.info("Negligible improvement in last rounds: declare convergence {} > {}"\
                        .format(n_blocked_iters, MAX_BLOCKED_ITERS))
            break

        if n_iter % 1000 == 0:
            print("OBJ: {}, p_nothing {}, p_streaming {}, T {}, n_iter {}, size {}, n_blocked_iters {}"\
                  .format(obj, p_nothing, p_streaming, temperature, n_iter, size, n_blocked_iters))


    obj = obj_function(bws, a, b)
    logger.debug("RESULT: {}".format(obj))

    return obj, n_users

N_SEEDS = 20
N_NOTHING = 4
p_streaming = 0.5

results = []
for s in range(N_SEEDS):
    np.random.seed(s)
    seed(s)

    for i, p_nothing in enumerate(np.linspace(0.1, 0.9, N_NOTHING)):
        logger.info("seed {}/{}, p_nothing {}/{}".format(s, N_SEEDS, i, N_NOTHING))

        obj, n_users = run_optimization(p_nothing, p_streaming)
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
