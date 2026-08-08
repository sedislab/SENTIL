<div align="center">

# SENTIL

#### The Julia package for Probabilistic Signal Temporal Logic

[![Julia](https://img.shields.io/badge/Julia-%E2%89%A51.10-blue.svg)](https://julialang.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

Julia bindings for the [`sentil`](https://github.com/sedislab/SENTIL/tree/main/sentil-core) engine. `] add Sentil` and `using Sentil`.

SENTIL has three main capabilities. Deterministic STL monitoring, offline over a recorded trace or streaming one sample at a time. Probabilistic monitoring, which fits a noise model to sensor data and estimates satisfaction probability with confidence bounds. And synthesis, from a specification to a control input to an online controller.

## Your first monitor

```julia
using Sentil

phi = formula("always (speed > 5)")
trace = Trace(collect(0.0:1.0:4.0), "speed", [12.0, 9.0, 7.0, 4.0, 6.0])
println("robustness ", robustness(phi, trace))   # -1.0
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin. The per-sample signal and the violated spans are one call away:

```julia
robustness_signal(phi, trace)
violations(phi, trace)
```

## Online streaming

An `OnlineMonitor` folds one timed reading at a time, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```julia
using Sentil

function watch()
    monitor = OnlineMonitor("always[0, 10] (x > -0.9)")
    for t in 0:59
        verdict = update!(monitor, Float64(t), Dict("x" => sin(t * 0.3)))
        if verdict.resolved && !verdict.satisfied
            return println("violated at t=$t, robustness=$(round(verdict.value, digits=3))")
        end
    end
    println("held over the whole stream")
end

watch()
```

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and returns the probability with a Wilson confidence interval.

```julia
using Sentil

trace = Trace(collect(0.0:1.0:19.0), "x", [0.4 + 0.05i for i in 0:19])

lifting = LiftingRegistry()
register_noise!(lifting, "x", gaussian(0.0, 0.3))

phi = formula("P>=0.9 (always (x > 0))")
result = check(phi, trace, lifting)
println("probability ", round(result.probability, digits=3),
        ", interval [", round(result.interval.lower, digits=3), ", ",
        round(result.interval.upper, digits=3), "], holds ", result.holds)
```

The noise model can be fit from calibration data with `fit_gaussian(residuals(truth, sensor))`, and `check_conservative` and `check_sequential` give the Clopper-Pearson interval and a sequential decision over the same ensemble.

## Specifications

The premade library is on the Julia side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Build a formula straight from one.

```julia
using Sentil

spec = SpecBuilder("automotive/safe_following_distance")
phi = build_formula(with_param(spec, "rho", 1.0))
```

List them with `available_specs()`, or browse them under [`specifications/`](https://github.com/sedislab/SENTIL/tree/main/specifications).

## Benchmarks

The package carries the same engine as every other binding, so Julia runs at the core's speed. These plots put Julia and the Rust core against the baseline tools, from the same runs.

![Online streaming cost per sample: SENTIL (Julia) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_julia.png)

Per-sample streaming cost across the bindings, with the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (Julia) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_julia.png)

Offline cost over the trace length, Julia and the core against RTAMT, MoonLight, and Banquo.

![Memory: the SENTIL engine streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](https://github.com/sedislab/SENTIL/tree/main/benchmarks), and all the results are in [`docs/CLAIMS.md`](https://github.com/sedislab/SENTIL/blob/main/docs/CLAIMS.md).

## Install

### Package manager

From the Julia General Registry, at the Pkg REPL (press `]`):

```julia
add Sentil
```

or from a script:

```julia
import Pkg
Pkg.add("Sentil")
```

That pulls the compiled core in as a per-platform artifact, so there is nothing else to install.

### Prebuilt release

To load a core you fetched yourself, download the archive for your platform from the [releases page](https://github.com/sedislab/SENTIL/releases), named `sentil-<version>-<triple>-julia.tar.gz` so the x86_64 Linux one is `sentil-<version>-x86_64-linux-gnu-julia.tar.gz`, unpack it, and point `SENTIL_LIB` at the library before `using Sentil`.

#### Linux

```sh
export SENTIL_LIB="/path/to/libsentil.so"
```

#### macOS

```sh
export SENTIL_LIB="/path/to/libsentil.dylib"
```

#### Windows

From PowerShell:

```powershell
$env:SENTIL_LIB = "C:\path\to\sentil.dll"
```

### Build from source

Building the core needs a Rust toolchain. Clone the repository, build `libsentil`, and develop the package against it:

```sh
git clone https://github.com/sedislab/SENTIL
cd SENTIL
cargo build --release -p sentil-ffi
export SENTIL_LIB="$PWD/target/release/libsentil.so"
```

```julia
import Pkg
Pkg.develop(path="sentil-jl")
Pkg.test("Sentil")
```

## Contributing

Point `SENTIL_LIB` at a freshly built `libsentil`, then run the suite:

```julia
import Pkg
Pkg.test("Sentil")
```

The pull-request flow is in the repository [CONTRIBUTING.md](https://github.com/sedislab/SENTIL/blob/main/CONTRIBUTING.md).

## Documentation

The [documentation site](https://sentil.pages.dev) has the guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/start/tutorial). The `examples/` directory has several examples to get you started.

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](https://github.com/sedislab/SENTIL/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/sedislab/SENTIL/blob/main/LICENSE-APACHE), at your option.