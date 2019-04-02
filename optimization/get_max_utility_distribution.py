import logging

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

from heuristic_solution import run_optimization as heuristic_optimizer
from traditional_solution import run_optimization as traditional_optimizer

p_nothing = 0.5
p_streaming = 0.5

logger = logging.getLogger('aachen_net.org')
logger.setLevel(logging.DEBUG)
logger.propagate = False

formatter = logging.Formatter("%(asctime)s::%(levelname)s::%(module)s::%(message)s",
                              "%Y-%m-%d %H:%M:%S")

ch = logging.StreamHandler()
ch.setLevel(logging.DEBUG)
ch.setFormatter(formatter)
logger.addHandler(ch)

objs = []

for seed in range(100):
    obj, _, _ = heuristic_optimizer(p_nothing, p_streaming, logger, topology_seed=14, search_seed=seed)
    objs.append(obj)

pd.DataFrame(objs).to_csv("ciao.csv", index=None, header=None)
