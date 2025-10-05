import numpy as np
import pytest

import sentil
from sentil import Formula, SystemModel, Bounds, Backend, SafetyFilter, Controller, synthesis

def model_and_bounds():
    model = SystemModel.linear([[1.0]], [[1.0]], [1.0], ["x"], 1.0, 3)
    bounds = Bounds([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])
    return model, bounds

def test_smooth_and_numerics():
    t = sentil.Trace([0, 1, 2], {"x": [1.0, 2.0, 0.5]})
    assert Formula.parse("always (x > 0)").smooth_robustness(t) > 0
    assert synthesis.soft_min([3.0, 1.0, 2.0], 100.0) < 1.5
    sol = synthesis.solve_spd([[4.0, 1.0], [1.0, 3.0]], [1.0, 2.0])
    assert 4 * sol[0] + sol[1] == pytest.approx(1.0)

def test_synthesize():
    model, bounds = model_and_bounds()
    assert model.input_dimension == 3
    res = synthesis.synthesize(model, Formula.parse("always (x > 0)"), bounds,
                               backend=Backend.Gradient)
    assert isinstance(res.input, np.ndarray) and len(res.input) == 3

def test_safety_filter_clamps():
    _, bounds = model_and_bounds()
    clamped = SafetyFilter(bounds).filter([2.0, 0.0, -2.0])
    assert clamped[0] <= 1.0 + 1e-9 and clamped[2] >= -1.0 - 1e-9

def test_controller_steps_and_drops():
    model, bounds = model_and_bounds()
    ctrl = Controller(model, Formula.parse("always (x > 0)"), 1, 1_000_000, bounds=bounds)
    assert len(ctrl.control([1.0])) >= 1
    del ctrl

def test_falsify_and_counterexample():
    model, bounds = model_and_bounds()
    g = Formula.parse("always (x > 0)")
    cx = g.find_counterexample(model, bounds, max_iters=50)
    assert hasattr(cx, "input") and hasattr(cx, "trace")

def test_chance_constraint():
    from sentil import SimExpr, SimModel, NoiseModel, ChanceConstraint
    walk = SimModel(["y"], 0.1, 10, [SimExpr.constant(0.0)],
                    [SimExpr.prev(0) + SimExpr.noise(0)], [NoiseModel.gaussian(0, 1)])
    chance = ChanceConstraint(Formula.parse("always (y < 5)"), 0.9)
    report = chance.validate(walk.to_stochastic_system(), samples=300)
    assert 0.0 <= report.estimate <= 1.0