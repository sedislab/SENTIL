import numpy as np
import pytest

import sentil
from sentil import Formula, Monitor, OnlineMonitor, MultiMonitor, FormulaBank, var

def trace():
    return sentil.Trace([0, 1, 2, 3], {"x": [1.0, -2.0, 3.0, -4.0]})

def test_formula_parse_json_introspect():
    phi = Formula.parse("always[0,5] (x > 0)")
    assert phi.is_temporal
    assert phi.variables == ["x"]
    assert Formula.from_json(phi.to_json()).to_json() == phi.to_json()

def test_operators_build_formulas():
    x = var("x")
    pred = (x * 2 - 1) > 5
    assert isinstance(pred, Formula)
    combined = pred.always(0, 3) & (x < 10).eventually(0, 2)
    assert isinstance(combined, Formula)
    assert "and" in str(combined)

def test_probability_wrap_and_guard():
    p = (var("x") > 0).always(0, 5).probability(0.9)
    assert "P" in str(p)
    with pytest.raises(sentil.EvaluationError):
        (var("x") > 0).probability(1.5)

def test_formula_robustness():
    g = Formula.parse("always (x > 0)")
    assert g.robustness(trace()) == pytest.approx(-4.0)
    assert len(g.robustness_signal(trace())) == 4
    assert all(hasattr(v, "start") for v in g.violations(trace()))

def test_trace_is_a_mapping():
    t = trace()
    assert len(t) == 4
    assert "x" in t and "y" not in t
    assert isinstance(t["x"], np.ndarray) and t["x"][3] == -4.0
    assert t.get("missing") is None
    assert list(t) == ["x"]
    with pytest.raises(KeyError):
        t["nope"]

def test_ring_buffer_is_a_sequence():
    rb = sentil.RingBuffer(3)
    for i in range(5):
        rb.push(float(i), float(i * i))
    assert len(rb) == 3 and rb.is_full
    assert rb[0] == (2.0, 4.0) and rb[-1] == (4.0, 16.0)
    assert [v for _, v in rb] == [4.0, 9.0, 16.0]
    assert rb.mean() == pytest.approx((4 + 9 + 16) / 3)
    with pytest.raises(IndexError):
        rb[99]

def test_monitor_offline_and_streaming():
    m = Monitor("always (x > 0)", sentil.Config(time=sentil.TimeMode.Discrete))
    assert m.robustness(trace()) == pytest.approx(-4.0)
    m2 = Monitor("x > 0")
    assert m2.update(0.0, {"x": 5.0}).satisfied

def test_online_multi_and_bank():
    om = OnlineMonitor("eventually (x > 2)")
    assert len(om.run(trace())) == 4

    mm = MultiMonitor()
    mm.add("safe", "x > 0")
    mm.add("big", Formula.parse("x > 2"))
    out = mm.update(0.0, {"x": 5.0})
    assert set(out) == {"safe", "big"} and out["safe"].satisfied

    bank = FormulaBank()
    bank.add("a", "always (x > 0)")
    bank.add("b", "eventually (x > 2)")
    assert set(bank.robustness(trace())) == {"a", "b"}