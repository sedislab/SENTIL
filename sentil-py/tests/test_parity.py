import pytest

from sentil import (
    Bounds,
    Expr,
    Formula,
    LiftingRegistry,
    Monitor,
    MultiMonitor,
    NoiseModel,
    OnlineMonitor,
    SemanticError,
    SimExpr,
    SimModel,
    SmcConfig,
    SpecBuilder,
    SystemModel,
    Trace,
)

def test_expr_math():
    assert (Expr.var("x").sqrt() > 2).to_json() == Formula.parse("sqrt(x) > 2").to_json()
    assert (Expr.var("x").min(0) < 1).to_json() == Formula.parse("min(x, 0) < 1").to_json()
    assert (Expr.var("x").max(Expr.var("y")) > 0).to_json() == Formula.parse(
        "max(x, y) > 0"
    ).to_json()
    assert (Expr.var("x") ** 2 > 4).to_json() == Formula.parse("x ^ 2 > 4").to_json()
    assert (Expr.var("x") % 3 == 0).to_json() == Formula.parse("x % 3 == 0").to_json()

def test_bounds_clamp():
    assert list(Bounds([-1, -1], [1, 1]).clamp([2.0, -3.0])) == [1.0, -1.0]

def test_spec_settings():
    for name in SpecBuilder.available():
        builder = SpecBuilder(name)
        smc = builder.smc_settings()
        if smc is not None:
            assert 0.0 < smc[0] < 1.0 and smc[1] > 0
        sprt = builder.sprt_settings()
        if sprt is not None:
            assert len(sprt) == 5
        ams = builder.ams_settings()
        if ams is not None:
            assert len(ams) == 2

def test_noise_from_file(tmp_path):
    model = NoiseModel.gaussian(1.5, 0.5)
    path = tmp_path / "model.json"
    path.write_text(model.to_json())
    assert NoiseModel.from_file(str(path)).to_json() == model.to_json()

def test_last_probability():
    monitor = Monitor("always (x > 0)")
    assert monitor.last_probability() is None
    monitor.update(0, {"x": 1.0})
    assert monitor.last_probability() is None
    with pytest.raises(SemanticError):
        Monitor("P>=0.8 (always (x > 0))").update(0, {"x": 1.0})

    lifting = LiftingRegistry()
    lifting.register("x", NoiseModel.gaussian(0.0, 1.0))
    online = OnlineMonitor.with_lifting(Formula.parse("P>=0.8 (x > 0)"), lifting)
    online.update(0, {"x": 1.0})
    # P(1 + N(0, 1) > 0) is the standard normal CDF at 1
    assert online.last_probability() == pytest.approx(0.8413, abs=0.02)

def test_online_probabilistic():
    lifting = LiftingRegistry()
    lifting.register("x", NoiseModel.gaussian(0.0, 0.3))
    phi = Formula.parse("P>=0.8 (always (x > 0))")
    monitor = OnlineMonitor.with_lifting(phi, lifting)
    monitor.update(0, {"x": 2.0})
    bank = MultiMonitor()
    bank.add("det", "x > 0")
    bank.add_probabilistic("prob", phi, lifting)
    verdicts = bank.update(0, {"x": 2.0})
    assert "det" in verdicts and "prob" in verdicts

    narrow = LiftingRegistry()
    narrow.register("x", NoiseModel.gaussian(0.0, 0.05))
    bounded = OnlineMonitor.with_lifting(
        Formula.parse("P>=0.95(always[0, 2](x > 0.35))"), narrow, SmcConfig(samples=200)
    )
    for t in (0, 1):
        verdict = bounded.update(t, {"x": 1.0})
        assert verdict.resolved is False
        assert bounded.last_probability() == 0.0
    verdict = bounded.update(2, {"x": 1.0})
    assert verdict.resolved is True
    assert bounded.last_probability() == 1.0

def test_smooth_gradients():
    trace = Trace.indexed(3)
    trace.add_signal("x", [3, 1, 2])
    value, gradient = Formula.parse("always[0,2](x > 0)").smooth_value_and_gradient(trace)
    assert isinstance(gradient, dict) and len(gradient["x"]) == 3
    model = SystemModel.linear([[1.0]], [[1.0]], [0.0], ["x"], 1.0, 4)
    _, per_input = Formula.parse("eventually[0,4](x > 2)").smooth_gradient(
        model, [0.0], [0.5, 0.5, 0.5, 0.5]
    )
    assert len(per_input) == 4

def test_simexpr_math():
    init = SimExpr.constant(0.5)
    advance = SimExpr.prev(0).sin() + SimExpr.noise(0).min(SimExpr.constant(0.5))
    model = SimModel(["x"], 0.1, 10, [init], [advance], [NoiseModel.gaussian(0.0, 0.1)])
    trace = model.simulate(42)
    assert len(trace["x"]) == 11