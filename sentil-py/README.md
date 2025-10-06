# sentil

Python bindings for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. The engine is a compiled Rust core shipped inside the wheel, so there is no Rust toolchain to install and nothing to build: `pip install sentil` and import it.

## Install

```
pip install sentil
```

The optional extras pull in their dependencies: `sentil[pandas]` for DataFrame ingest, `sentil[plotting]` for the robustness plots.

## A first monitor

```python
import sentil

trace = sentil.Trace([0, 1, 2, 3], {"speed": [12, 9, 7, 4]})
phi = sentil.Formula.parse("always (speed > 5)")
print(phi.robustness(trace))   # negative: the property is violated by the end
```

Formulas compose with Python operators, and predicates read the way you would write them:

```python
from sentil import var, always, eventually

phi = always(var("speed") > 5) & eventually(var("gap") > 2)
```

Traces behave like a mapping from variable name to a NumPy array:

```python
trace["speed"]        # the values, as an ndarray
"speed" in trace      # True
list(trace)           # the variable names
```

## Streaming

```python
monitor = sentil.OnlineMonitor("always (x > 0)")
for t, x in enumerate(stream):
    verdict = monitor.update(t, {"x": x})
    if not verdict.satisfied:
        alarm()
```

## Probabilistic monitoring

```python
noise = sentil.NoiseModel.gaussian(0.0, 0.3)
lifting = sentil.LiftingRegistry()
lifting.register("x", noise)
result = sentil.Formula.parse("P>=0.95 (always (x > 0))").check(trace, lifting)
print(result.probability, result.interval)
```

## Building from source

You need a Rust toolchain and [maturin](https://www.maturin.rs).

```
maturin develop --release   # build and install into the active environment
pytest tests/               # run the test suite
```

## Scope

The whole engine is here: STL and PrSTL monitoring offline and online, the statistical layer, synthesis, the specifications library, and the GPU path. A few low-level hooks that pass a Python callable into the Rust engine are not exposed, because the engine runs them across worker threads where holding the GIL is unsafe: optimizing an arbitrary Python objective with CMA-ES, a system model whose dynamics are a Python function, and a sequential test driven by a Python Bernoulli source. The declarative equivalents cover the same ground: build a `SimModel` or a `LinearModel`, synthesize against it, and check or falsify it.

## More

Per-language guides and runnable examples live at the documentation site. SENTIL is by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University, dual licensed under MIT or Apache-2.0.