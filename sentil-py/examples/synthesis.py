"""Synthesize a control input that satisfies a spec, then shield it online."""

from sentil import Bounds, Formula, SafetyFilter, SystemModel, synthesis

# x_{t+1} = x_t + u_t
model = SystemModel.linear([[1.0]], [[1.0]], [1.0], ["x"], 1.0, 3)
spec = Formula.parse("always (x > 0)")
bounds = Bounds([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])

result = synthesis.synthesize(model, spec, bounds)
print("input:", result.input, "robustness:", result.robustness, "holds:", result.holds)

shield = SafetyFilter(bounds)
print("shielded:", shield.filter([2.0, 0.5, -3.0]))