"""Offline robustness over a recorded trace, in discrete and dense time."""

import sentil
from sentil import Formula

trace = sentil.Trace([0, 1, 2, 3, 4], {"speed": [12.0, 9.0, 7.0, 4.0, 6.0]})
phi = Formula.parse("always (speed > 5)")

print("robustness:", phi.robustness(trace))
print("per sample:", phi.robustness_signal(trace))
print("violations:", [(v.start, v.end) for v in phi.violations(trace)])
print("dense robustness:", phi.robustness_dense(trace))