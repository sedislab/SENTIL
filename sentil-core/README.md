<div align="center">

# SENTIL

#### Runtime verification and controller synthesis for Probabilistic Signal Temporal Logic

[![Crates.io](https://img.shields.io/crates/v/sentil.svg)](https://crates.io/crates/sentil)
[![Documentation](https://docs.rs/sentil/badge.svg)](https://docs.rs/sentil)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

SENTIL is a complete tool for working with Signal Temporal Logic (STL) and its probabilistic extension PrSTL. The following are its main capabilities. Deterministic STL monitoring, that is offline over a recorded trace or streaming one sample at a time. Probabilistic monitoring, which fits a noise model to sensor data and estimates satisfaction probability with confidence bounds. And synthesis, which turns a specification into a control input and then into an online controller. They compose, so you can do monitoring and synthesis in turn or some other combination.

## Your first monitor

```rust
use sentil::{Formula, Trace};

fn main() -> Result<(), sentil::Error> {
    let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0, 4.0])?;
    trace.add_signal("speed", vec![12.0, 9.0, 7.0, 4.0, 6.0])?;

    let phi = Formula::parse("always (speed > 5)")?;
    println!("robustness {}", phi.robustness(&trace)?);
    // robustness -1
    Ok(())
}
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin.

## Online streaming

We can also do online monitoring or streaming that is at runtime, with samples arriving one after the other, we can check whether various formulas hold. A `Monitor` receives in one timed reading at a time and returns the current robustness, so you can watch a live system and stop/log the moment it breaks.

```rust
use sentil::{Monitor, MonitorConfig};

fn main() -> sentil::Result<()> {
    let mut monitor = Monitor::new("always[0, 10] (x > -0.9)", MonitorConfig::new())?;
    for t in 0..60 {
        let x = (f64::from(t) * 0.3).sin();
        let verdict = monitor.update(f64::from(t), &[("x", x)])?;
        if verdict.is_resolved() && verdict.value() < 0.0 {
            println!("violated at t={t}, robustness={:.3}", verdict.value());
            return Ok(());
        }
    }
    println!("held over the whole stream");
    Ok(())
}
```

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor, and SENTIL lifts every reading into an ensemble of candidate trajectories, evaluates the formula on each, and reports the satisfaction probability with a confidence interval.

```rust
use sentil::{LiftingRegistry, Monitor, MonitorConfig, NoiseInteraction, NoiseModel, SmcConfig, Trace};

fn main() -> sentil::Result<()> {
    let times: Vec<f64> = (0..20).map(f64::from).collect();
    let mut trace = Trace::new(times)?;
    trace.add_signal("x", (0..20).map(|i| 0.4 + 0.05 * f64::from(i)).collect::<Vec<_>>())?;

    let mut lifting = LiftingRegistry::new();
    lifting.register("x", NoiseModel::gaussian(0.0, 0.3)?, NoiseInteraction::Additive);

    let config = MonitorConfig::new().smc(SmcConfig { samples: 5000, ..SmcConfig::default() });
    let monitor = Monitor::new("P>=0.9 (always (x > 0))", config)?;

    let result = monitor.check(&trace, &lifting)?;
    println!(
        "probability {:.3}, interval [{:.3}, {:.3}], holds {}",
        result.probability, result.interval.lower, result.interval.upper, result.holds
    );
    Ok(())
}
```

## Specifications

Writing a correct temporal-logic formula for those new to the verification world is a well-documented failure mode, so SENTIL ships vetted specifications across ten domains: aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, and UAV. Each is a parameterized template with a plain-language description, a citation to the standard or paper it comes from, default parameters you can override, and both a deterministic and a probabilistic form. Reach for one instead of writing your own. And you can also save formulas you use often.

```rust
use sentil::SpecRegistry;

fn main() -> sentil::Result<()> {
    // RSS safe-following-distance, from the ISO 26262 and Mobileye RSS references.
    let phi = SpecRegistry::global()
        .builder("automotive/safe_following_distance")?
        .with_param("rho", 1.0)? // the follower's reaction time, in seconds
        .build_formula()?;

    println!("{phi}");
    Ok(())
}
```

Browse the set under [`specifications/`](../specifications), or list them at runtime with `SpecRegistry::global().available()`.

## Benchmarks

SENTIL runs in real-time loops, so speed is the point. These plots come from the suite in `benchmarks/`, which also ships the runners that reproduce them and the baseline tools they compare against.

![Discrete-time offline robustness against the STL baseline tools](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/discrete_offline.png)

Offline robustness over a long discrete-time trace, SENTIL against the STL baseline tools.

![Streaming throughput and per-sample latency](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_online.png)

Sustained streaming throughput and the per-sample latency distribution, tail included.

![Memory, the sliding window against a whole-trace monitor](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory.png)

Memory over the length of the trace: the deque holds a window, the offline tools hold the whole stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### Package manager
From crates.io, the same on Windows, macOS, and Linux:

```bash
cargo add sentil
```

### Prebuilt release
For a pre-release or the repository tip, add a git dependency:

```toml
sentil = { git = "https://github.com/sedislab/SENTIL" }
```

### Build from source
From a source checkout:

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL
cargo build -p sentil
```

## Features

The default build carries the monitor, the statistical layer, synthesis, the specifications library, and the file readers. Turn features off for a leaner build, or on for the optional backends.

| Feature | Default | What it adds |
| --- | --- | --- |
| `std` | yes | the standard library; drop it for a `no_std` STL monitor |
| `statistical` | yes | Monte Carlo, confidence intervals, SPRT, rare-event splitting |
| `synthesis` | yes | smooth robustness, open-loop and receding-horizon synthesis |
| `specs` | yes | the premade PrSTL specifications library |
| `ingest` | yes | traces from CSV, TSV, and MATLAB files |
| `parallel` | yes | Monte Carlo across cores with Rayon |
| `sqlite` | yes | traces from a SQLite table |
| `gpu` | no | the WebGPU rare-event and synthesis-batching path |
| `parquet`, `arrow` | no | Parquet and Arrow or Feather traces |
| `hdf5`, `mcap` | no | ingest formats that need a system library |

For a bare STL monitor with none of the statistical or synthesis machinery:

```bash
cargo add sentil --no-default-features --features std
```

## Examples

We provide very minimal startup examples:

```
cargo run --example offline_monitoring
cargo run --example online_streaming
cargo run --example probabilistic
cargo run --example synthesis
```

## Documentation

The full API reference is on [docs.rs](https://docs.rs/sentil). The [documentation site](https://sentil.pages.dev) carries the guides, the specification syntax, and a per-language tutorial for each binding, and the long-form [tutorial](https://sentil.pages.dev/docs/start/tutorial) walks through monitoring, probabilistic checking, and synthesis end to end. The Python, C and C++, Java, Julia, MATLAB, CLI, ROS, and embedded packages all wrap this crate and reproduce its numbers against a shared deterministic oracle.

## Contributing

Contributions are welcome, from a fixed typo to a whole new binding. The C ABI and every language binding build against this crate, so run its tests and the linter for a change:

```
cargo test -p sentil
cargo clippy --all-targets
```

The pull-request flow, and a table of which packages need Rust and which only a prebuilt core, is in [CONTRIBUTING.md](../CONTRIBUTING.md).

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