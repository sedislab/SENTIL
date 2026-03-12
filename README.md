<div align="center">

# SENTIL

#### Runtime verification and controller synthesis for Probabilistic Signal Temporal Logic

[![Crates.io](https://img.shields.io/crates/v/sentil.svg)](https://crates.io/crates/sentil)
[![PyPI](https://img.shields.io/pypi/v/sentil.svg)](https://pypi.org/project/sentil)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

[Documentation](https://sentil.pages.dev) | [Reproduce the claims](docs/REPRODUCE.md) | [Paper](https://arxiv.org/abs/2605.21676)

</div>

SENTIL decides whether a system meets a Signal Temporal Logic specification, and it does three things you can use on their own or chain together. Deterministic STL monitoring computes the quantitative robustness of a trace, offline or one streaming sample at a time. Probabilistic monitoring fits a noise model to sensor data and estimates how likely a PrSTL specification holds, with a confidence bound. Synthesis turns a specification into a control input and then into an online controller. It is a Rust engine with a stable C ABI, wrapped by a package in every major language and a command-line tool, and each package carries the compiled engine inside it, so a Python user with no Rust and no GPU installs a wheel and it runs.

![Offline discrete STL robustness, SENTIL against the baseline tools](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/discrete_offline.png)

Offline robustness over a long discrete-time trace, SENTIL against the STL baseline tools. The streaming monitor holds a flat per-sample cost and memory proportional to the formula's windows, not the length of the stream, so it sustains a real-time loop where a whole-trace monitor cannot. The numbers, and how to regenerate them, are in [docs/CLAIMS.md](docs/CLAIMS.md).

## Try it

The command-line tool is the fastest way in. Install it, then check a trace against a formula:

```bash
brew install sedislab/sentil/sentil     # or scoop, winget, cargo install
sentil check -f 'always[0, 10] (speed < 30)' -t drive.csv
```

It prints the robustness, positive and equal to the margin while the property holds, negative and equal to the shortfall when it fails, and its exit code is the verdict, so `sentil check ... && deploy` runs only when the spec held. The [CLI guide](sentil-cli) covers the verbs, the file formats, and the exit codes.

## Install

SENTIL lives in your language. Each package ships the compiled engine, so none of them needs the Rust toolchain, with the one honest exception of the microcontroller target, which cannot host the statistical or GPU path and says so.

| Surface | Install | Guide |
| --- | --- | --- |
| Rust, the core | `cargo add sentil` | [sentil-core](sentil-core) |
| Python | `pip install sentil` | [sentil-py](sentil-py) |
| C and C++ | vcpkg, Conan, or a release archive | [sentil-cpp](sentil-cpp), [sentil-ffi](sentil-ffi) |
| Java | Maven `io.github.sedislab:sentil` | [sentil-java](sentil-java) |
| Julia | `] add Sentil` | [sentil-jl](sentil-jl) |
| MATLAB and Simulink | the File Exchange | [sentil-matlab](sentil-matlab) |
| Command line | Homebrew, Scoop, Winget, or a release | [sentil-cli](sentil-cli) |
| ROS 2 | `apt install ros-<distro>-sentil-ros` | [sentil-ros](sentil-ros) |
| Microcontrollers | Arduino, PlatformIO, ESP-IDF, Zephyr, bare metal | [sentil-embedded](sentil-embedded) |
| Apollo | a Cyber RT module in your workspace | [sentil-apollo](sentil-apollo) |
| AUTOSAR Adaptive | a CMake build against your platform | [sentil-autosar-adaptive](sentil-autosar-adaptive) |

Every package reads the same formula, reproduces the same robustness against a shared oracle, and raises the engine's errors in its own idiomatic type. Pick your language, follow the guide linked above, and the [documentation site](https://sentil.pages.dev) has the long form for each.

## What you can do

- Monitor a recorded trace or a live stream against an STL formula and read the signed robustness with the intervals where it fails. [Start here](sentil-core).
- Ask a probabilistic question. `P>=0.95 (always[0, 10] (gap > 5))` estimates the satisfaction probability with a Wilson or Clopper-Pearson interval, and a sequential test decides a hypothesis with bounded error. [Start here](sentil-core).
- Reach for a vetted specification instead of writing one, from a library of premade PrSTL specs across ten domains, each with a citation and default parameters. [Browse them](specifications).
- Synthesize a control input or an online controller that satisfies a specification, and shield a nominal controller with a control-barrier filter. [Start here](sentil-core).
- Run it where the system runs: [ROS 2](sentil-ros), [Apollo](sentil-apollo), [AUTOSAR Adaptive](sentil-autosar-adaptive), and [microcontrollers](sentil-embedded) down to a Cortex-M0+.

## Reproduce the claims

SENTIL goes into safety-critical loops, so every number it reports is checkable. [docs/CLAIMS.md](docs/CLAIMS.md) maps each performance and correctness claim to the command that regenerates it, the value from the reference run, and the tolerance it holds to, and [docs/REPRODUCE.md](docs/REPRODUCE.md) walks the tiers. The CPU tier runs in one step, with no GPU and no downloads:

```bash
make verify                                                          # with the toolchain on the host
docker compose -f docker/docker-compose.yml run --rm sentil-verify   # or fully offline in Docker
```

The deterministic-oracle set under [benchmarks/](benchmarks) is the cross-language ground truth: every binding reproduces the same robustness at every sample, and continuous integration fails the build on a tolerance drift.

## Documentation

The [documentation site](https://sentil.pages.dev) carries a guide for each language, the formula and specification syntax, interactive lessons, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial). The Rust API reference is on [docs.rs](https://docs.rs/sentil), and every package ships the same worked programs in its own `examples/` directory: offline monitoring in discrete and dense time, an online streaming run, a probabilistic check, and a synthesis run.

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

## Contributing

Contributions are welcome, from a fixed typo to a whole new binding. The C ABI and every language binding build against the core crate, so run its tests and the linter for a change:

```bash
cargo test -p sentil
cargo clippy --all-targets
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the pull-request flow and a table of which packages need Rust and which only a prebuilt core.

## License

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University. It is dual licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.