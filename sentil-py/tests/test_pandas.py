import pytest

pd = pytest.importorskip("pandas")

import sentil
from sentil.pandas import trace_from_dataframe

def test_reads_time_and_signals():
    df = pd.DataFrame({"time": [0.0, 1.0, 2.0], "speed": [12.0, 9.0, 7.0]})
    trace = trace_from_dataframe(df)
    phi = sentil.Formula.parse("always (speed > 5)")
    assert phi.robustness(trace) == pytest.approx(2.0)

def test_custom_time_column_and_selected_values():
    df = pd.DataFrame({"t": [0, 1, 2], "x": [1.0, 2.0, 3.0], "y": [0.0, 0.0, 0.0]})
    trace = trace_from_dataframe(df, time_column="t", value_columns=["x"])
    assert sentil.Formula.parse("always (x > 0)").robustness(trace) == pytest.approx(1.0)

def test_missing_time_column_raises():
    df = pd.DataFrame({"x": [1.0, 2.0]})
    with pytest.raises(KeyError):
        trace_from_dataframe(df)

def test_missing_value_column_raises():
    df = pd.DataFrame({"time": [0, 1], "x": [1.0, 2.0]})
    with pytest.raises(KeyError):
        trace_from_dataframe(df, value_columns=["absent"])