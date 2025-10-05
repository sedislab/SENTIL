import pytest

import sentil
from sentil import Formula, Monitor, ParseError, SemanticError, EvaluationError, SentilError

def test_error_hierarchy():
    assert issubclass(ParseError, SentilError)
    assert issubclass(SemanticError, SentilError)
    assert issubclass(EvaluationError, SentilError)

def test_parse_error_points_at_the_input():
    with pytest.raises(ParseError):
        Formula.parse("always (")

def test_unknown_variable_is_semantic():
    trace = sentil.Trace([0, 1], {"y": [1.0, 2.0]})
    with pytest.raises(SemanticError):
        Monitor("always (x > 0)").robustness(trace)

def test_evaluation_failures_match_the_other_bindings():
    trace = sentil.Trace([0, 1], {"x": [1.0, 2.0]})
    with pytest.raises(EvaluationError):
        Monitor("always (frobnicate(x) > 0)").robustness(trace)
    with pytest.raises(EvaluationError):
        Monitor("always (sqrt(x, 2) > 0)").robustness(trace)

def test_bad_noise_raises_not_crashes():
    with pytest.raises(SentilError):
        sentil.NoiseModel.gaussian(0.0, -1.0)

def test_zero_capacity_ring_buffer_raises():
    with pytest.raises(SentilError):
        sentil.RingBuffer(0)

def test_non_monotonic_trace_raises():
    with pytest.raises(SentilError):
        sentil.Trace([0.0, 1.0, 0.5])

def test_json_and_file_failures_are_evaluation_errors():
    with pytest.raises(EvaluationError):
        Formula.from_json("{not json")
    with pytest.raises(EvaluationError):
        sentil.NoiseModel.from_json("{not json")
    with pytest.raises(EvaluationError) as caught:
        sentil.NoiseModel.from_file("/no/such/model.json")
    assert "/no/such/model.json" in str(caught.value)