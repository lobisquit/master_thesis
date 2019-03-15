import numpy as np
from docplex.mp.model import Model
from graph_tool.all import *

import cplex

g = load_graph('data/aachen_net/abstract_topology.graphml')

# setup cplex model
m = Model(log_output=True)
m.parameters.workmem = 2048
m.parameters.mip.display = 2
m.parameters.mip.interval = -1

bw = {}
bwr = {}
u = {}

MAX_DSLAM_BW = 10
MAX_ROUTER_BW = 10
MAX_MAINFRAME_BW = 10

def name(source, target=None, var='x'):
    if target:
        return "{}_{}~{}".format(var, source, target)
    else:
        return "{}_{}".format(var, source)

print("Start")

obj_func = 0
mainframe_bw = 0

# setup bandwith and its requirements from users
n_leaves = 0
for v in g.vertices():
    if g.vertex(v).in_degree() == 0:
        n_leaves += 1
        print("Leaf {}".format(v), end='\r')

        bw[v] = m.continuous_var(name=name(v, var='bw'))
        bwr[v] = 1 # TODO set proper value

        # define utility for current user
        u[v] = m.binary_var(name=name(v, var='u'))

        m.add_constraint(u[v] <= bw[v] / bwr[v],
                         ctname="utility def {}".format(v))

        obj_func += u[v]
        mainframe_bw += bw[v]

print('n_leaves = {}'.format(n_leaves))

# objective function
m.set_objective('max', obj_func)

# flow conditions
mainframe = np.where(g.vp['root'].a)[0][0]

# mainframe serves everybody: sum all user bandwith
m.add_constraint(mainframe_bw <= MAX_MAINFRAME_BW,
                 ctname="mainframe {} bw limit".format(mainframe))

print("Setup mainframe", end='\r')

routers = g.get_in_edges(mainframe)[:, 0]
for router in routers:
    print("Router {}".format(router))

    # count total bandwith crossing this router
    router_bw = 0

    dslams = g.get_in_edges(router)[:, 0]
    for dslam in dslams:
        print("DSLAM {}".format(dslam), end='\r')

        users = g.get_in_edges(dslam)[:, 0]

        dslam_bw = sum(bw[v] for v in users)
        m.add_constraint(dslam_bw <= MAX_DSLAM_BW,
                         ctname="dslam {} bw limit".format(dslam))

        # mark the contribution towards the router
        router_bw += dslam_bw

    m.add_constraint(router_bw <= MAX_ROUTER_BW,
                     ctname="router {} bw limit".format(router))

logfile = "logs/unnamed.log"

with open(logfile, "w") as f:
    solver = m.solve(log_output=f)

    if solver is None:
        logger.error("Unable to solve ILP")
        exit(1)
    else:
        m.print_solution()
