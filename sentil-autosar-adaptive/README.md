# sentil-autosar-adaptive

Runtime verification and controller synthesis for the AUTOSAR Adaptive Platform, built on the SENTIL engine and exposed over ara::com.

Two applications ship in one package. `sentil_monitor` subscribes to a signal frame, evaluates Signal Temporal Logic and probabilistic STL specifications, and offers a verdict over ara::com: a Verdict field for late subscribers and a Violation event on a falsifying edge. `sentil_control` offers a ComputeControl method that shields a nominal command into the bounds or plans one from the spec. Both link the compiled SENTIL core, so the ECU needs no Rust toolchain.

The data path is events and fields, and the control plane is methods, because a SOME/IP round trip per sensor tick is the wrong shape for a real-time monitor. The signal arrives as a streamed event; the verdict is a notified field.

## Running it without a vendor platform

The package ships an open-source ara::com over vsomeip, so the apps and tests run on a plain Linux box with no Adaptive Platform license. Build against the stub:

```
cmake -B build -G Ninja -DSENTIL_AP_VENDOR=stub \
  -DCMAKE_TOOLCHAIN_FILE=cmake/toolchains/linux-x86_64-stub.cmake \
  -DSENTIL_ROOT=/path/to/sentil/install
ninja -C build
ctest --test-dir build
```

`ctest` runs the parity check (the verdict the app serves equals the engine on the cross-language oracle) and the ARXML validation (the model is well-formed and the service-instance ids are unique). The lifecycle check, which exercises the service-discovery handshake over SOME/IP, needs a running routing manager and is built only with `-DSENTIL_BUILD_LIFECYCLE_TEST=ON`.

A vendor Adaptive Platform is a toolchain swap, not a code change: set `-DSENTIL_AP_VENDOR=vector|eb|apex` with the vendor toolchain file and point `ARA_COM_ROOT` at the vendor ara::com. The app source is unchanged; `cmake/FindAraCom.cmake` locates the vendor binding and `codegen/generate.sh` runs the vendor generator on `model/*.arxml`.

## Installing from a GitHub release

To build without cloning, download the source archive for the tag you want from the GitHub release and build it the same as the from-source path above. Tags are at https://github.com/sedislab/SENTIL/releases.

Linux and macOS:

```
curl -L -o sentil-v0.3.0.tar.gz https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.tar.gz
tar xzf sentil-v0.3.0.tar.gz
cd SENTIL-0.3.0/sentil-autosar-adaptive
```

Windows (PowerShell):

```
Invoke-WebRequest https://github.com/sedislab/SENTIL/archive/refs/tags/v0.3.0.tar.gz -OutFile sentil-v0.3.0.tar.gz
tar xzf sentil-v0.3.0.tar.gz
cd SENTIL-0.3.0\sentil-autosar-adaptive
```

From there run the same cmake configure and build as above, against the stub or a vendor Adaptive Platform. For an on-ECU install, the `.deb` and `.rpm` place the apps, manifests, and the compiled SENTIL core under `/opt/sentil`; that layout is documented in `packaging/opt-layout`.

## The service interfaces

Three interfaces live in `model/`, separate from their deployment in `manifest/` per the AUTOSAR methodology split.

`SignalInterface` carries a `SignalFrame` event with a timestamp and parallel name and value arrays, the monitor's input. `VerdictInterface` offers a `Verdict` field (getter and notifier, no setter), a `Violation` event that fires only on a satisfied-to-violated transition, and a `SetSpecification` method for control-plane reconfiguration. `ControlInterface` offers a `ComputeControl` method, a `ControlCommand` event for the streaming case, and a `ControllerStatus` field exposing deadline-met, feasibility, and whether the safety filter intervened.

The field-to-variable mapping lives in whatever produces the `SignalFrame`, an adapter or the perception service, so the monitor reads named scalars and does not parse vendor message types.

## Deployment

The deployment is three manifests per app, authored as JSON in `manifest/` (a vendor generator emits ARXML from the same fields). The machine manifest sets the unicast address, the service-discovery multicast and port, and the function groups. Each execution manifest binds an executable to a process, its function-group states, and its scheduling. Each service-instance manifest maps every interface to its SOME/IP service, instance, event, and event-group ids, rendered into `manifest/vsomeip/*.json` so the stub resolves them; `VerdictInterface` is service `0x6001`, `ControlInterface` is `0x6002`, and the shared `SignalInterface` is `0x6000`. On a hard-real-time target set the SENTIL thread count to one for a tighter worst-case execution time.

## Safety framing

The monitor is a Platform Health Management Supervised Entity: it reports a checkpoint each cycle and raises a Diagnostic Event Management event on a sustained violation, so a blown deadline or a falsified property is caught by the platform's health machinery. The controller reports its cycle checkpoints the same way.

SENTIL is a verdict source and a synthesis engine, not the safety case. The core is a QM element; a downstream safety monitor decides what to do with a verdict, and the integrator owns the ISO 26262 argument. The Lean-proved monotonic deque is offered as formal-methods evidence for the streaming monitor, not as a safety certification.

## Synthesis surface

The control application carries the whole synthesis subsystem. Online, `ComputeControl` runs the receding-horizon controller or the safety-filter shield within a deadline. Offline, the same application plans an open-loop input sequence over the horizon, searches for a counterexample input that violates the spec, and runs a chance-constraint check against the model under Gaussian process noise. These are the design-time companions to the online controller, sharing one engine and one model, so a plan validated offline is the policy the controller runs online.

## Examples

`examples/adas_fca_monitor` is a self-contained forward-collision demo: a perception publisher, the monitor, and a planner that consumes the verdict, all over the stub on one box, with its own vsomeip config and a one-command `run.sh`. The lead vehicle closes in, the follow-distance verdict flips, and the planner logs the violation. The control side follows the same shape: a client calls `sentil_control`'s `ComputeControl` over ara::com to shield a nominal command, then actuates the result.

## Contributing

The deterministic tests build with no transport, against a staged core at `SENTIL_ROOT`:

```
cmake -S sentil-autosar-adaptive -B build -DSENTIL_AP_VENDOR=none -DSENTIL_ROOT=/opt/sentil
cmake --build build
ctest --test-dir build --output-on-failure
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab. Dual licensed under MIT or Apache-2.0.