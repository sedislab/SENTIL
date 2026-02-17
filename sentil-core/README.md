# sentil

The SENTIL engine: the Rust core that every other binding wraps. It monitors Signal Temporal Logic and its probabilistic extension PrSTL, and it synthesizes inputs and controllers that satisfy a specification. The crate is published as `sentil` on crates.io.

Three capabilities live here and each stands alone: deterministic STL monitoring, offline and streaming; probabilistic statistical monitoring, with noise models and confidence bounds; and synthesis, from a spec to a control input to an online controller. They compose, and the heavier layers sit behind feature flags so a minimal build carries only the streaming monitor.

## Quickstart

```rust
use sentil::{Formula, Trace};

fn main() -> Result<(), sentil::Error> {
    let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0, 4.0])?;
    trace.add_signal("speed", vec![12.0, 9.0, 7.0, 4.0, 6.0])?;

    let phi = Formula::parse("always (speed > 5)")?;
    println!("robustness {}", phi.robustness(&trace)?);
    Ok(())
}
```

## Install

From crates.io, the same on Windows, macOS, and Linux:

```
cargo add sentil
```

To pin a pre-release or work against the repository, add a git or path dependency in `Cargo.toml`:

```toml
sentil = { git = "https://github.com/sedislab/SENTIL" }
```

To build from a source checkout, clone the repository and build the workspace:

```
git clone https://github.com/sedislab/SENTIL
cd SENTIL
cargo build -p sentil
```

## Features

The default build carries the monitor, the statistical layer, synthesis, the specifications library, and the file readers. Turn features off for a leaner build, or on for the optional backends.

| Feature | On by default | What it adds |
| --- | --- | --- |
| `std` | yes | the standard library; drop it for a `no_std` STL monitor |
| `statistical` | yes | Monte Carlo, confidence intervals, SPRT, rare-event splitting |
| `synthesis` | yes | smooth robustness, open-loop and receding-horizon synthesis |
| `specs` | yes | the premade PrSTL specifications library |
| `ingest` | yes | reading traces from CSV, TSV, and MATLAB files |
| `parallel` | yes | spreading Monte Carlo across cores with Rayon |
| `sqlite` | yes | reading traces from a SQLite table |
| `gpu` | no | the WebGPU rare-event and synthesis batching path |
| `parquet`, `arrow` | no | reading Parquet and Arrow/Feather traces |
| `hdf5`, `mcap` | no | ingest formats that need a system library |

A bare STL monitor, with none of the statistical or synthesis machinery:

```
cargo add sentil --no-default-features --features std
```

## Examples

The canonical set runs from the crate:

```
cargo run --example offline_monitoring
cargo run --example online_streaming
cargo run --example probabilistic
cargo run --example synthesis
```

## More

The full API reference is on [docs.rs](https://docs.rs/sentil), and the guides and tutorials are on the documentation site. SENTIL is dual licensed under MIT or Apache-2.0.