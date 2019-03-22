from random import random

import numpy as np
from numpy.random import normal, rand

MAX_DSLAM_BW = 1e9 # bps
MAX_ROUTER_BW = 10e9 # bps

STREAM_PROBS = [0.85, 0.10, 0.05]

STREAM_PARAMS = [ (-3.035, -.5061),
                  (-4.850, -.6470),
                  (-17.53, -1.048) ]

STREAM_BW = [
    5e6,
    3e6,
    1e6,
]

# obtained via `find_web_parameters.py`
WEB_BW = 500e3
WEB_PARAMS = (-14.98544276, -0.87800541)

def utility(x, a, b):
    return a * np.power(x, b) + 1

def obj_function(bw, a, b):
    active_mask = a != 0

    utilities = utility(bw[active_mask], a[active_mask], b[active_mask])
    return np.log(utilities).mean()

def get_realization(p_nothing, p_streaming):
    # no user here
    if random() < p_nothing:
        # this combinarion of params makes obj_function 0 (irrelevant)
        # for each value of x
        return 0, 0, 0

    # streaming user (conditional probability)
    if random() < p_streaming:
        # randomly pick the three possible streamers
        profile = np.random.choice(3, p=STREAM_PROBS)
        return STREAM_PARAMS[profile] + (STREAM_BW[profile], )
    else:
        # a b c
        return WEB_PARAMS + (WEB_BW, )
