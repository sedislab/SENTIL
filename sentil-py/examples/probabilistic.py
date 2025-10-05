"""Probabilistic monitoring: lift a noisy sensor and estimate satisfaction."""

import sentil
from sentil import Formula, LiftingRegistry, NoiseModel, SmcConfig

trace = sentil.Trace(list(range(20)), {"x": [0.4 + 0.05 * i for i in range(20)]})
lifting = LiftingRegistry()
lifting.register("x", NoiseModel.gaussian(0.0, 0.3))

phi = Formula.parse("P>=0.9 (always (x > 0))")
result = phi.check(trace, lifting, SmcConfig(samples=5000))
print(
    f"probability {result.probability:.3f}, "
    f"interval [{result.interval.lower:.3f}, {result.interval.upper:.3f}], "
    f"holds {result.holds}"
)