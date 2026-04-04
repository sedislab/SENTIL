"""The shared deterministic oracle, the same fixture every other binding runs."""

import json
import math
import os
import struct

import pytest

import sentil

ORACLE = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "benchmarks", "deterministic", "oracle.json",
)

def decode(token):
    """One oracle value."""
    if token == "inf":
        return math.inf
    if token == "-inf":
        return -math.inf
    if token == "nan":
        return math.nan
    return float(token)

def same_bits(a, b):
    return struct.pack("<d", a) == struct.pack("<d", b)

def cases():
    with open(ORACLE, encoding="utf-8") as handle:
        return json.load(handle)["deterministic"]

@pytest.mark.parametrize("case", cases(), ids=lambda c: c["id"])
def test_the_oracle_robustness_is_reproduced_exactly(case):
    trace = sentil.Trace.indexed(case["length"])
    for signal in case["signals"]:
        trace.add_signal(signal["name"], [decode(v) for v in signal["values"]])

    monitor = sentil.Monitor(case["formula"])
    got = monitor.robustness_signal(trace)
    expected = [decode(v) for v in case["expected"]]

    assert len(got) == len(expected)
    for i, (g, e) in enumerate(zip(got, expected)):
        assert same_bits(float(g), e), (
            f"{case['id']} sample {i}: got {g!r}, oracle says {e!r}"
        )

def test_the_whole_oracle_set_is_present():
    assert len(cases()) >= 44