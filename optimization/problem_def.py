from random import random

import numpy as np
from numpy.random import normal, rand

MAX_DSLAM_BW = 1e9 # bps
MAX_ROUTER_BW = 10e9 # bps
MAX_MAINFRAME_BW = 10000e9 # bps

BW_TOLERANCE_WEB = 200e3 # bps
BW_TOLERANCE_STREAM = 1e6 # bps

BW_MIN_WEB = 500e3 # bps
BW_MIN_STREAM = 5e6 # bps

MARGIN = 0.95

def utility(value, critic_value, tolerance, margin):
    exponent = (value - critic_value) / tolerance
    return 1 / (1 + ((1 - margin) / margin) ** exponent)

def obj_function(bws, bws_min, tolerances, margins):
    active_mask = bws_min != 0
    utilities = utility(bws[active_mask],
                        bws_min[active_mask] + tolerances[active_mask],
                        tolerances[active_mask],
                        margins[active_mask])

    return np.log(utilities).mean()

def get_realization(p_nothing, p_streaming):
    # no user here
    if random() < p_nothing:
        # this combinarion of params makes obj_function 0 (irrelevant)
        # for each value of x
        return 0, 0, 0

    # streaming user (conditional probability)
    if random() < p_streaming:
        return BW_MIN_STREAM, BW_TOLERANCE_STREAM, MARGIN
    else:
        return BW_MIN_WEB, BW_TOLERANCE_WEB, MARGIN
