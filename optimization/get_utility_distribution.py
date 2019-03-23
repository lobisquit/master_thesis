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

results = []

for p_streaming in np.linspace(0.1, 0.9, 4):
    for p_nothing in np.linspace(0.1, 0.9, 4):
        _, _, h_utilities = heuristic_optimizer(p_nothing, p_streaming, logger)
        results.append(pd.DataFrame({
            'p_streaming': p_streaming,
            'p_nothing': p_nothing,
            'utility': h_utilities,
            'type': 'Heuristic'
        }))

        _, _, t_utilities = traditional_optimizer(p_nothing, p_streaming, logger)
        results.append(pd.DataFrame({
            'p_streaming': p_streaming,
            'p_nothing': p_nothing,
            'utility': t_utilities,
            'type': 'Traditional'
        }))

pd.concat(results).to_csv('../data/optimization/utility_distribution.csv.gz')
