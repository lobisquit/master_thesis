import itertools
import logging
from random import seed

import numpy as np
import pandas as pd
from graph_tool.all import *

from problem_def import *

cache = {}
def get_subtree_leaves(g, v):
    if v in cache:
        return cache[v]

    if isinstance(v, Vertex):
        v = int(v)

    if g.vertex(v).in_degree() == 0:
        cache[v] = v
        return [v]
    else:
        child_leaves = []
        for child, _, _ in g.get_in_edges(v):
            child_leaves += get_subtree_leaves(g, child)

        cache[v] = child_leaves
        return child_leaves

def run_optimization(p_nothing, p_streaming, logger):
    g = load_graph('../data/aachen_net/abstract_topology.graphml')

    g.vp['bw'] = g.new_vertex_property("double")
    g.vp['expected_bw'] = g.new_vertex_property("double")

    # utility params
    g.vp['a'] = g.new_vertex_property("double")
    g.vp['b'] = g.new_vertex_property("double")

    g.vp['bw'].a = np.nan
    g.vp['a'].a = np.nan
    g.vp['b'].a = np.nan
    g.vp['expected_bw'].a = np.nan

    leaves = [int(v) for v in g.vertices() if g.vertex(v).in_degree() == 0]
    a, b, expected_bw = zip(*[
        get_realization(p_nothing, p_streaming)
        for _ in leaves
    ])

    n_users = np.count_nonzero(a)

    g.vp['a'].a[leaves] = a
    g.vp['b'].a[leaves] = b
    g.vp['expected_bw'].a[leaves] = expected_bw

    g.vp['bw'].a[leaves] = g.vp['expected_bw'].a[leaves]

    mainframe = np.where(g.vp['root'].a)[0][0]
    routers = g.get_in_edges(mainframe)[:, 0]
    dslams = list(itertools.chain.from_iterable(
        [g.get_in_edges(router)[:, 0] for router in routers]
    ))

    for dslam in dslams:
        dslam_leaves = g.get_in_edges(dslam)[:, 0]
        total_bw = sum(g.vp['bw'].a[dslam_leaves])

        if total_bw == 0:
            g.vp['bw'][dslam] = 0
        else:
            # remodulate to reach limit (either up or down)
            g.vp['bw'][dslam] = MAX_DSLAM_BW
            g.vp['bw'].a[dslam_leaves] *= MAX_DSLAM_BW / total_bw

    for router in routers:
        router_leaves = get_subtree_leaves(g, router)
        total_bw = sum(g.vp['bw'].a[router_leaves])

        if total_bw == 0:
            g.vp['bw'][router] = 0
        else:
            if total_bw > MAX_ROUTER_BW:
                # remodulate to reach limit, but only down
                # since DSLAMs are already maxed out
                g.vp['bw'][router] = MAX_ROUTER_BW
                g.vp['bw'].a[router_leaves] *= MAX_ROUTER_BW / total_bw

    # NOTE no constraints on mainframe

    # compute objective function

    obj = obj_function(g.vp['bw'].a[leaves],
                     g.vp['a'].a[leaves],
                     g.vp['b'].a[leaves])

    utilities = utility(g.vp['bw'].a[leaves], a, b)
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
        '../data/optimization/traditional.csv',
        index=None
    )
