# Sentil.jl

Julia bindings for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. The package wraps the same compiled core the C, C++, and Python bindings use, so a Julia program gets the full engine: deterministic STL monitoring, probabilistic statistical monitoring, and synthesis, with no Rust toolchain required.

## What it gives you

Parse or compose a formula, evaluate its robustness over a trace offline or one sample at a time online, estimate how likely a probabilistic specification holds with rigorous confidence bounds, and synthesize an input or a controller that satisfies a specification. The specifications library ships vetted, standards-derived formulas you can reach for directly.

```julia
using Sentil

phi = formula("always[0, 10](speed < 120)")
trace = Trace(collect(0.0:1.0:5.0), "speed", [100.0, 110.0, 125.0, 118.0, 90.0, 80.0])
robustness(phi, trace)            # the margin to the boundary, negative when violated
violations(phi, trace)            # the time spans where it fails
```

## Installing

From the Julia General Registry, once the package is published:

```julia
import Pkg
Pkg.add("Sentil")
```

That pulls in the compiled core as an artifact, so nothing else is needed.

To build from source, compile the core and point `SENTIL_LIB` at it:

```sh
cargo build --release -p sentil-ffi
export SENTIL_LIB="$PWD/target/release/libsentil.so"   # .dylib on macOS, .dll on Windows
```

Then `using Sentil` finds the library through `SENTIL_LIB`. This is the path to use while developing against an uncommitted core.

## Examples

The `examples/` directory has one runnable script per capability: `offline_discrete.jl` and `offline_dense.jl` for offline monitoring, `online_streaming.jl` for the streaming monitor, `prstl.jl` for probabilistic monitoring, and `synthesis.jl` for going from a specification to a controller. Run one with `julia --project examples/prstl.jl`.

## Documentation

The full guide, with the per-language API reference and worked lessons, is on the documentation site. The other bindings expose the same operations under the same names, so an example in any of them reads across.

## Errors

Every fallible call raises a typed exception rather than crashing: `ParseError` for a malformed formula, `SemanticError` for an input the engine cannot make sense of, and `EvaluationError` for a runtime fault. Catch `SentilError` to handle any of them. A closed handle raises on use rather than reaching freed memory.

## License

Dual licensed under MIT or Apache 2.0, at your option. See the repository root for the full texts.

## Authors

Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University.
