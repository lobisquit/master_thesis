import matplotlib.pyplot as plt
from scipy.optimize import *

from problem_def import *

data = [
    (64, 0.61),
    (128, 0.79),
    (256, 0.88),
    (512, 0.96),
    (1024, 0.955),
    (2048, 0.96),
]

data_x, data_y = zip(*data)

params, cov = curve_fit(utility, data_x, data_y)

plt.xscale('log')
fit_x = np.logspace(np.log10(min(data_x)), np.log10(max(data_x)))
plt.plot(fit_x, utility(np.array(fit_x), *params))
plt.plot(data_x, data_y)
plt.show()

print(params)
