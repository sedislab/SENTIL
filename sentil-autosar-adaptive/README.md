<div align="center">

# SENTIL

#### The AUTOSAR Adaptive Platform integration for Probabilistic Signal Temporal Logic

[![AUTOSAR Adaptive](https://img.shields.io/badge/AUTOSAR-Adaptive%20Platform-blue.svg)](https://www.autosar.org)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

AUTOSAR Adaptive Platform bindings for the [`sentil`](../sentil-core) engine, exposed over ara::com. Two Adaptive Applications build from one CMake project. `sentil_monitor` subscribes to a signal frame, evaluates Signal Temporal Logic and probabilistic STL, and offers a verdict over the bus, a `Verdict` field for late subscribers and a `Violation` event on a falsifying edge. `sentil_control` shields a nominal command into its bounds or plans one from a specification. Both link the compiled core, so the ECU needs no Rust toolchain.

SENTIL has three separable capabilities. Deterministic STL monitoring, folding one signal frame at a time into a verdict. Probabilistic monitoring, which fits a noise model to a sensor reading and estimates satisfaction probability with a confidence interval. And synthesis, from a specification to a control command to an online receding-horizon controller.

The data path is events and fields, and the control plane is methods. A SOME/IP round trip on every sensor tick is the wrong shape for a real-time monitor, so the signal arrives as a streamed `SignalFrame` event and the verdict is a notified `Verdict` field that late subscribers latch. `SetSpecification` and `ComputeControl` are the only request-response calls.

## Run the forward-collision demo

The package ships an open-source ara::com stub over vsomeip, so the two apps and the example run on a plain Linux box with no Adaptive Platform license. Build against the stub, then start the demo:

```
cmake -B build -G Ninja -DSENTIL_AP_VENDOR=stub \
  -DCMAKE_TOOLCHAIN_FILE=cmake/toolchains/linux-x86_64-stub.cmake \
  -DSENTIL_ROOT=/path/to/sentil/install
ninja -C build
examples/adas_fca_monitor/run.sh
```

`run.sh` starts the monitor, which hosts the routing manager, a perception publisher, and a planner as three processes on one box, each pointed at its vsomeip config. The publisher pushes a `front_gap` that closes half a meter each tick from 30 down. The monitor checks `front_gap > 5.0` and notifies the verdict, and the planner latches it and engages a fallback when the `Violation` event fires:

```
verdict t=0.0 robustness=25.000 satisfied=1
verdict t=0.1 robustness=24.500 satisfied=1
...
VIOLATION at t=5.1, engaging fallback
```

The robustness starts at `25.0`, the gap of 30 minus the 5 meter bound. It falls toward zero as the lead vehicle closes in, and at `t = 5.1` the gap is 4.5 and the property fails by `0.5`, so the `Violation` event fires once and the planner switches to its fallback. The vsomeip and `SENTIL_ROOT` prerequisites the build needs are under [Install](#install).

## The service interfaces

Three interfaces live in `model/` as ARXML, separate from their deployment in `manifest/` per the AUTOSAR methodology split.

`SignalInterface` carries a `SignalFrame` event with a timestamp and parallel name and value arrays, the monitor's input. `VerdictInterface` offers a `Verdict` field (getter and notifier, no setter), a `Violation` event that fires only on a satisfied-to-violated transition, and a `SetSpecification` method for control-plane reconfiguration. `ControlInterface` offers a `ComputeControl` method, a `ControlCommand` event for the streaming case, and a `ControllerStatus` field exposing deadline-met, feasibility, and whether the safety filter intervened.

The field-to-variable mapping lives in whatever produces the `SignalFrame`, an adapter or the perception service, so the monitor reads named scalars and never parses vendor message types. The `SetSpecification` request carries a spec string; a parse failure returns a rejection byte and logs the diagnostic rather than swapping in a broken monitor.

## Probabilistic monitoring

The shipped `sentil_monitor` starts on the deterministic spec `front_gap > 5.0`. Set `SENTIL_MONITOR_MODE=probabilistic` before launch to register the PrSTL form without a rebuild:

```
SENTIL_MONITOR_MODE=probabilistic VSOMEIP_APPLICATION_NAME=sentil_monitor build/sentil_monitor
```

This registers `P>=0.95 (front_gap > 5.0)` and lifts `front_gap` under additive Gaussian sensor noise, `NoiseModel::gaussian(0.0, 0.5)`, over 2000 samples. The `Verdict` field then carries the estimated satisfaction probability with its Wilson interval in the `probability`, `ci_lower`, and `ci_upper` slots, and `satisfied` reflects whether the estimate clears the `0.95` threshold. Any SENTIL noise family works through the programmatic path, and the Gaussian lift here is one choice among them.

## Synthesis

The control application carries the whole synthesis subsystem. `sentil_control` defaults to a least-restrictive safety filter that shields a nominal command into its per-input bounds. Set `SENTIL_CONTROL_MODE=synthesize` to swap in the receding-horizon controller over a double integrator with the spec `always[0, 20] (pos > 1.0 and pos < 9.0)`:

```
SENTIL_CONTROL_MODE=synthesize VSOMEIP_APPLICATION_NAME=sentil_control build/sentil_control
```

A client calls `ComputeControl` over ara::com and reads the command back with the `ControllerStatus` field. Offline, the same application plans an open-loop input sequence over the horizon, searches for a counterexample input that falsifies the spec, and runs a chance-constraint check against the model under Gaussian process noise, so a plan validated at design time is the policy the controller runs online. The `ControllerStatus` feasibility flag reports that a command was produced within the deadline, not that the spec is proven to hold; the online monitor is what confirms the property as the system runs.

## Deployment

The deployment is three manifests per app, authored as JSON in `manifest/`, with a vendor generator emitting ARXML from the same fields. The machine manifest sets the unicast address, the service-discovery multicast and port, and the function groups. Each execution manifest binds an executable to a process, its function-group states, and its scheduling. Each service-instance manifest maps every interface to its SOME/IP service, instance, event, and event-group ids, rendered into `manifest/vsomeip/*.json` so the stub resolves them. `VerdictInterface` is service `0x6001`, `ControlInterface` is `0x6002`, and the shared `SignalInterface` is `0x6000`. On a hard-real-time target, set the SENTIL thread count to one for a tighter worst-case execution time.

## Safety framing

The monitor is a Platform Health Management Supervised Entity: it reports a checkpoint each cycle and raises a Diagnostic Event Management event on a sustained violation, so a blown deadline or a falsified property reaches the platform's health machinery. The controller reports its cycle checkpoints the same way.

SENTIL is a verdict source and a synthesis engine, not the safety case. The core is a QM element; a downstream safety monitor decides what to do with a verdict, and the integrator owns the ISO 26262 argument. The Lean-proved monotonic deque is offered as formal-methods evidence for the streaming monitor, not as a safety certification. The apps monitor at the core's per-sample cost; the measured numbers are in [`benchmarks/`](../benchmarks) and [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

This integration is a CMake build against the stub or a vendor Adaptive Platform, not a package-manager install. The build produces the two Adaptive Applications and their manifests for an AP project.

### Build against the stub

The stub links vsomeip, which is not bundled here. Install its dependencies first. vsomeip needs Boost from the system package manager (`libboost-all-dev` on Debian or Ubuntu, `boost-devel` on Fedora), and vsomeip itself is built from the [COVESA source](https://github.com/COVESA/vsomeip) and installed, which places the `vsomeip3` CMake package the configure step resolves.

`SENTIL_ROOT` points at a core install tree: `lib/libsentil.so` plus the C and C++ headers under `include/`. Build the core once from the workspace root with `cargo build --release -p sentil-ffi`, then stage the artifacts into a prefix, copying `target/release/libsentil.so` into `lib/` and both `sentil-ffi/include/sentil.h` and `sentil-cpp/include/sentil` into `include/`. Point `SENTIL_ROOT` at that prefix and configure:

```
cmake -B build -G Ninja -DSENTIL_AP_VENDOR=stub \
  -DCMAKE_TOOLCHAIN_FILE=cmake/toolchains/linux-x86_64-stub.cmake \
  -DSENTIL_ROOT=/path/to/sentil/install
ninja -C build
ctest --test-dir build
```

`ctest` runs the parity check, the verdict the app serves equals the engine on the cross-language oracle, and the ARXML validation, the model is well-formed and the service-instance ids are unique. The lifecycle check exercises the service-discovery handshake over SOME/IP, needs a running routing manager, and is built only with `-DSENTIL_BUILD_LIFECYCLE_TEST=ON`.

The stub build and the examples are Linux only. macOS and Windows are reached through a vendor Adaptive Platform and its toolchain.

### Use a vendor Adaptive Platform

A vendor platform is a toolchain swap, not a code change. Set `-DSENTIL_AP_VENDOR=vector|eb|apex` with the vendor toolchain file and point `ARA_COM_ROOT` at the vendor ara::com. The app source is unchanged. `cmake/FindAraCom.cmake` locates the vendor binding and `codegen/generate.sh` runs the vendor generator on `model/*.arxml`.

### From a GitHub release

To build without cloning, download the source archive for the tag you want and build it the same way. Tags are on the [releases page](https://github.com/sedislab/SENTIL/releases).

```
curl -L -o sentil-v0.3.0.tar.gz https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.tar.gz
tar xzf sentil-v0.3.0.tar.gz
cd SENTIL-0.3.0/sentil-autosar-adaptive
```

For an on-ECU layout, the packaging under `packaging/opt-layout` places the apps, manifests, and the compiled core under `/opt/sentil`.

## Documentation

The [documentation site](https://sentil.pages.dev) carries the guides and the [AUTOSAR integration overview](https://sentil.pages.dev/docs/autosar). The `examples/adas_fca_monitor` directory holds the full forward-collision demo, a perception publisher, the monitor, and a planner that consumes the verdict, all over the stub on one box with its own vsomeip config and a one-command `run.sh`.

## Contributing

The deterministic tests build with no transport, against a staged core at `SENTIL_ROOT`, which is the tier continuous integration runs:

```
cmake -S sentil-autosar-adaptive -B build -DSENTIL_AP_VENDOR=none -DSENTIL_ROOT=/opt/sentil
cmake --build build
ctest --test-dir build --output-on-failure
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

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

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.