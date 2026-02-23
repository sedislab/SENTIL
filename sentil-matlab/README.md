# SENTIL for MATLAB

MATLAB and Simulink bindings for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. The toolbox wraps the same compiled core the C, C++, Python, Julia, and Java bindings use, so MATLAB gets the full engine: deterministic STL monitoring, probabilistic statistical monitoring, and synthesis. The `sentil` package is the programmatic API; the `SENTIL Monitor` Simulink block runs the streaming monitor inside a model.

## What it gives you

Parse or build a formula, evaluate its robustness over a trace offline or one sample at a time online, estimate how likely a probabilistic specification holds with a confidence interval, and synthesize an input or a controller that satisfies a specification. The specifications library ships vetted, standards-derived formulas you can reach for by name.

```matlab
phi = sentil.Formula.parse('always[0, 10](speed < 120)');
trace = sentil.Trace(0:5, 'speed', [100 110 125 118 90 80]);
phi.robustness(trace)     % the margin to the boundary, negative when violated
phi.violations(trace)     % the time spans where it fails
```

## Installing

From the MATLAB File Exchange, install the packaged toolbox by opening `Sentil.mltbx` in MATLAB, or with `matlab.addons.toolbox.installToolbox('Sentil.mltbx')`. The toolbox ships the compiled core inside it, so no Rust toolchain and no separate library are needed.

## Installing from a GitHub release

Download `Sentil.mltbx` from the assets on the GitHub release and install it by double-clicking it in MATLAB, or with `matlab.addons.toolbox.installToolbox('Sentil.mltbx')`. The published artifact carries the Linux MEX gateway and its library, so it installs and runs as is on Linux; on macOS or Windows the gateway is not in the package, so build it from source with `build_sentil` below.

## Building from source

The toolbox compiles the core and the MEX gateway in one step. From this directory, with a C toolchain and the Rust toolchain on the path:

```matlab
build_sentil
```

That builds `libsentil` with Cargo if it is not already present, then compiles the MEX gateway and the Simulink S-Function against it. Add the directory to the path and the `sentil` package resolves:

```matlab
addpath(pwd)
sentil.version()
```

The binding works on MATLAB R2021b and newer. The MEX gateway uses the C MEX API, which avoids a libstdc++ version dependency, so the compiled artifact loads on the MATLAB it was built against.

## Examples

The `examples/` directory has one script per capability: `offline_monitoring` for offline robustness in discrete and dense time, `online_streaming` for the streaming monitor, `probabilistic` for PrSTL monitoring with a confidence interval, and `synthesis` for going from a specification to a controller and a safety filter. Each prints the same results its counterpart in the other bindings does. Run one with `addpath(pwd, fullfile(pwd, 'examples')); offline_monitoring`.

## Simulink

`blocks/create_sentil_library` builds a masked block library with a `SENTIL Monitor` block. Drop the block into a model, set the formula and the input variables in the mask, and the block emits the running robustness for the signal wired to its input. The deterministic mode outputs the scalar robustness; the probabilistic mode lifts the signal and outputs the estimate with its bounds. `examples/run_fda_insulin_benchmark` drives the block over the UVA/Padova artificial-pancreas model across a 1000-patient cohort. That case study's reported figures were produced on MATLAB R2023a or newer, which the closed-loop model needs for its noise and solver behavior to match; the harness builds and runs on earlier releases, but the cohort numbers track the release.

## Tests

`runtests('tests')` runs the suite. `test_oracle` loads the shared `benchmarks/deterministic/oracle.json` and checks that every case reproduces the reference robustness bit for bit, which is the cross-language correctness gate. `test_sentil` covers each capability against the values the other bindings produce.

## Documentation

The full guide, with the per-language API reference and worked lessons, is on the documentation site. The other bindings expose the same operations under the same names, so an example in any of them reads across.

## Errors

Every fallible call throws an `MException` rather than crashing, with an identifier under the `sentil:` namespace: `sentil:parse` for a malformed formula, `sentil:semantic` for an input the engine cannot make sense of, and a descriptive message that points at the offending construct or states both sizes of a dimension mismatch. A handle used after `delete` throws rather than reaching freed memory.

## Contributing

Build the MEX artifacts and run the tests from MATLAB:

```
cd('sentil-matlab'); build_sentil; addpath(pwd); runtests('tests')
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## License

Dual licensed under MIT or Apache 2.0, at your option. See the repository root for the full texts.

## Authors

Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University.