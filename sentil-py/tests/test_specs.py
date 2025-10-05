import pytest

import sentil
from sentil import SpecBuilder

def test_registry_lists_specs():
    names = SpecBuilder.available()
    assert isinstance(names, list) and len(names) > 0

def test_build_a_spec():
    builder = SpecBuilder(SpecBuilder.available()[0])
    assert isinstance(builder.build_deterministic(), str)
    assert hasattr(builder.build_formula(), "robustness")
    assert isinstance(builder.parameters(), dict)

def test_unknown_spec_raises():
    with pytest.raises(sentil.SentilError):
        SpecBuilder("no/such/spec")