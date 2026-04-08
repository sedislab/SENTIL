<div align="center">

# SENTIL

#### The MATLAB and Simulink toolbox for probabilistic Signal Temporal Logic

[![MATLAB](https://img.shields.io/badge/MATLAB-%E2%89%A5R2021b-blue.svg)](https://www.mathworks.com)
[![Simulink](https://img.shields.io/badge/Simulink-S--Function%20block-blue.svg)](#simulink)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

MATLAB and Simulink bindings for the [`sentil`](../sentil-core) engine. The toolbox carries the full compiled core, so you don't need any Rust. The `sentil` package is the programmatic API; the `SENTIL Monitor` block runs the streaming monitor inside a Simulink model.

SENTIL has three main capabilities. Deterministic STL monitoring, offline over a recorded trace or streaming one sample at a time. Probabilistic monitoring, which fits a noise model to sensor data and estimates satisfaction probability with confidence bounds. And synthesis, from a specification to a control input to an online controller.

## Your first monitor

```matlab
trace = sentil.Trace([0 1 2 3 4], 'speed', [12 9 7 4 6]);
phi = sentil.Formula.parse('always (speed > 5)');
phi.robustness(trace)   % -1.0
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin. The per-sample signal and the violated spans are one call away:

```matlab
phi.robustness_signal(trace)   % the robustness at each sample
phi.violations(trace)          % the [start, end] spans where it fails
```

A `Formula`, `Trace`, `Monitor`, and the other native-backed classes are handles freed by `delete`; value types like the streaming verdict are plain structs. Every fallible call throws an `MException` under the `sentil:` namespace, `sentil:parse` for a malformed formula and `sentil:semantic` for an input the engine cannot make sense of, each with a message that points at the offending construct.

## Online streaming

An `OnlineMonitor` folds one timed reading at a time, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```matlab
monitor = sentil.OnlineMonitor('always[0, 10] (x > -0.9)');
for t = 0:59
    verdict = monitor.update(t, struct('x', sin(t * 0.3)));
    if verdict.resolved && ~verdict.satisfied
        fprintf('violated at t=%d, robustness=%.3f\n', t, verdict.value);
        break
    end
end
```

`satisfied` only carries a verdict once `resolved` is true; until then the monitor is still filling the window, so bound the horizon on a future-time operator.

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and returns the probability with a Wilson confidence interval.

```matlab
times = 0:19;
trace = sentil.Trace(times, 'x', 0.4 + 0.05 * times);

lifting = sentil.LiftingRegistry();
lifting.register('x', sentil.NoiseModel.gaussian(0.0, 0.3));

phi = sentil.Formula.parse('P>=0.9 (always (x > 0))');
config = sentil.SmcConfig;
config.samples = 5000;

result = phi.check(trace, lifting, config);
fprintf('probability %.3f, interval [%.3f, %.3f], holds %d\n', ...
    result.probability, result.interval.lower, result.interval.upper, result.holds);
```

## Specifications

The premade library is on the MATLAB side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Build a formula straight from one.

```matlab
spec = sentil.SpecBuilder('automotive/safe_following_distance');
phi = spec.with_param('rho', 1.0).build_formula();   % the follower's reaction time
% phi monitors gap, v_r, and v_f against the RSS safe-distance bound
```

List them with `sentil.SpecBuilder.available()`, or browse them under [`specifications/`](../specifications).

## Simulink

The streaming monitor runs inside a model through an S-Function block, so a controller can be checked against a specification as the simulation steps. Build the block library once:

```matlab
create_sentil_library("sentil_lib");   % writes sentil_lib.slx with a SENTIL Monitor block
```

Drag the `SENTIL Monitor` block from `sentil_lib` into your model and open its mask, or configure it from a script:

```matlab
add_block('sentil_lib/SENTIL Monitor', 'my_model/Monitor');
set_param('my_model/Monitor', ...
    'formula_str', 'always (speed < 120)', ...
    'var_names_str', 'speed', ...   % the input port carries these, in this order
    'mode_sel', 'Deterministic');   % or 'Probabilistic/SMC' with a Monte Carlo sample count
```

Wire the monitored signals to the input port in the order named. In deterministic mode the output port carries the running robustness at each step; in probabilistic mode it lifts the signal and carries four values, the robustness verdict, the running satisfaction probability, and the confidence bounds on that probability. The probability and its bounds belong to a `P[~p]` operator, so a formula written without one still reports robustness on the first output and holds the other three at NaN.

The worked example is the artificial-pancreas case study. `run_fda_insulin_benchmark` drives the block over the UVA/Padova insulin model across a 1000-patient cohort, checking that blood glucose stays in range with high probability and rarely goes hypoglycemic:

```matlab
addpath(pwd, fullfile(pwd, 'examples'))
run_fda_insulin_benchmark   % assembles the closed-loop model and drives the monitors over the cohort
```

The cohort figures that case study reports were produced on R2023a or newer, which the closed-loop model needs for its noise and solver behavior to match; the harness builds and runs on earlier releases, but the reported numbers track the release.

## Benchmarks

The toolbox carries the same engine as every other binding, so MATLAB runs at the core's speed. These plots put MATLAB and the Rust core against the baseline tools, from the same runs.

![Online streaming cost per sample: SENTIL (MATLAB) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_matlab.png)

Per-sample streaming cost across the bindings, with the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (MATLAB) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_matlab.png)

Offline cost over the trace length, MATLAB and the core against RTAMT, MoonLight, and Banquo.

![Memory: the SENTIL engine streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### Package manager

From the [MATLAB File Exchange](https://www.mathworks.com/matlabcentral/fileexchange), install the packaged toolbox by opening `Sentil.mltbx` in MATLAB, or from the command window:

```matlab
matlab.addons.toolbox.installToolbox('Sentil.mltbx')
```

### Prebuilt release

Download `Sentil.mltbx` from the assets on the [GitHub release](https://github.com/sedislab/SENTIL/releases) and install it the same way. The published package carries the gateway for Linux, macOS, and Windows, so it installs and runs as is on all three with no Rust toolchain and no build step.

### Build from source

The toolbox compiles the core and the MEX gateway in one step. You need a C compiler and a Rust toolchain on the path.

```matlab
% from a shell: git clone https://github.com/sedislab/SENTIL && cd SENTIL/sentil-matlab
build_sentil
addpath(pwd)
sentil.version()
```

`build_sentil` builds `libsentil` with Cargo if it is not already present, then compiles the MEX gateway and the Simulink S-Function against it. The binding works on MATLAB R2021b and newer; the MEX gateway uses the C MEX API, so you don't need libstdc++.

## Contributing

Build the MEX artifacts and run the tests from MATLAB:

```matlab
cd('sentil-matlab'); build_sentil; addpath(pwd); runtests('tests')
```

`test_oracle` loads the shared `benchmarks/deterministic/oracle.json` and checks that every case reproduces the reference robustness. The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The [documentation site](https://sentil.pages.dev) has the guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/start/tutorial). The `examples/` directory has example scripts, plus the Simulink insulin benchmark.

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

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.