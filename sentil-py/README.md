<div align="center">

# SENTIL

#### The Python package for probabilistic Signal Temporal Logic

[![PyPI](https://img.shields.io/pypi/v/sentil.svg)](https://pypi.org/project/sentil/)
[![Python](https://img.shields.io/pypi/pyversions/sentil.svg)](https://pypi.org/project/sentil/)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

Python bindings for the [`sentil`](../sentil-core) engine. `pip install sentil` and import it.

SENTIL has three main capabilities. Deterministic STL monitoring, offline over a recorded trace or streaming one sample at a time. Probabilistic monitoring, which fits a noise model to sensor data and estimates satisfaction probability with confidence bounds. And synthesis, from a specification to a control input to an online controller.

## Your first monitor

```python
import sentil

trace = sentil.Trace([0, 1, 2, 3, 4], {"speed": [12, 9, 7, 4, 6]})
phi = sentil.Formula.parse("always (speed > 5)")
print(phi.robustness(trace))   # -1.0
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin. Formulas compose with Python operators, and a trace behaves like a mapping from a name to a NumPy array:

```python
from sentil import var, always, eventually

phi = always(var("speed") > 5) & eventually(var("gap") > 2)
trace["speed"]        # the values, as an ndarray
```

## Online streaming

An `OnlineMonitor` folds one timed reading at a time, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```python
import math
import sentil

monitor = sentil.OnlineMonitor("always[0, 10] (x > -0.9)")
for t in range(60):
    x = math.sin(t * 0.3)
    verdict = monitor.update(float(t), {"x": x})
    if verdict.resolved and not verdict.satisfied:
        print(f"violated at t={t}, robustness={verdict.value:.3f}")
        break
else:
    print("held over the whole stream")
```

`satisfied` only carries a verdict once `resolved` is true; until then the monitor is still filling the window, so bound the horizon on a future-time operator.

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and returns the probability with a Wilson confidence interval.

```python
import sentil
from sentil import Formula, LiftingRegistry, NoiseModel, SmcConfig

trace = sentil.Trace(list(range(20)), {"x": [0.4 + 0.05 * i for i in range(20)]})

lifting = LiftingRegistry()
lifting.register("x", NoiseModel.gaussian(0.0, 0.3))

phi = Formula.parse("P>=0.9 (always (x > 0))")
result = phi.check(trace, lifting, SmcConfig(samples=5000))
print(f"probability {result.probability:.3f}, "
      f"interval [{result.interval.lower:.3f}, {result.interval.upper:.3f}], "
      f"holds {result.holds}")
```

## Specifications

The premade library is on the Python side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Reach for one instead of writing your own.

```python
import sentil

phi = (
    sentil.SpecBuilder("automotive/safe_following_distance")
    .with_param("rho", 1.0)  # the follower's reaction time
    .build_formula()
)
# phi monitors gap, v_r, and v_f against the RSS safe-distance bound
```

List them with `sentil.SpecBuilder.available()`, or browse them under [`specifications/`](../specifications).

## Benchmarks

These plots put Python and the Rust core against the baseline tools, from the same runs.

![Online streaming cost per sample: SENTIL (Python) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_python.png)

Per-sample streaming cost across the bindings, with the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (Python) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_python.png)

Offline cost over the trace length, Python and the core against RTAMT, MoonLight, and Banquo.

![Memory: SENTIL (Python) streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory_python.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### Package manager

From PyPI, with pip:

```bash
pip install sentil
```

or with uv:

```bash
uv add sentil
```

The optional extras pull their dependencies: `sentil[pandas]` for DataFrame ingest, `sentil[plotting]` for the robustness plots.

### Prebuilt release

Every tagged release attaches prebuilt wheels to the [GitHub release](https://github.com/sedislab/SENTIL/releases); download the one for your Python and platform and install it directly.

#### Linux and macOS

The shell expands the wildcard to the file you downloaded:

```bash
pip install ./sentil-0.3.0-*.whl
```

#### Windows

Pass the exact filename, since the shell leaves the wildcard unexpanded:

```bash
pip install .\sentil-0.3.0-cp311-cp311-win_amd64.whl
```

### Build from source

You need a Rust toolchain and [maturin](https://www.maturin.rs).

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL/sentil-py
maturin develop --release   # build and install into the active environment
python -m pytest tests
```

## Contributing

Build the extension into your environment and run the suite:

```bash
maturin develop --release
python -m pytest tests
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The [documentation site](https://sentil.pages.dev) carries the guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Signal Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.