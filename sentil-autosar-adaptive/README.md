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

## The service interfaces

Three interfaces live in `model/`, separate from their deployment in `manifest/` per the AUTOSAR methodology split.

`SignalInterface` carries a `SignalFrame` event with a timestamp and parallel name and value arrays, the monitor's input. `VerdictInterface` offers a `Verdict` field (getter and notifier, no setter), a `Violation` event that fires only on a satisfied-to-violated transition, and a `SetSpecification` method for control-plane reconfiguration. `ControlInterface` offers a `ComputeControl` method, a `ControlCommand` event for the streaming case, and a `ControllerStatus` field exposing deadline-met, feasibility, and whether the safety filter intervened.

The field-to-variable mapping lives in whatever produces the `SignalFrame`, an adapter or the perception service, so the monitor reads named scalars and does not parse vendor message types.

## Deployment

The deployment is three manifests per app, authored as JSON in `manifest/` (a vendor generator emits ARXML from the same fields). The machine manifest sets the unicast address, the service-discovery multicast and port, and the function groups. Each execution manifest binds an executable to a process, its function-group states, and its scheduling. Each service-instance manifest maps every interface to its SOME/IP service, instance, event, and event-group ids, rendered into `manifest/vsomeip/*.json` so the stub resolves them; `VerdictInterface` is service `0x6001`, `ControlInterface` is `0x6002`, and the shared `SignalInterface` is `0x6000`. On a hard-real-time target set the SENTIL thread count to one for a tighter worst-case execution time.

## Safety framing

The monitor is a Platform Health Management Supervised Entity: it reports a checkpoint each cycle and raises a Diagnostic Event Management event on a sustained violation, so a blown deadline or a falsified property is caught by the platform's health machinery. The controller reports its cycle checkpoints the same way.

SENTIL is a verdict source and a synthesis engine, not the safety case. The core is a QM element; a downstream safety monitor decides what to do with a verdict, and the integrator owns the ISO 26262 argument. The Lean-proved monotonic deque is offered as formal-methods evidence for the streaming monitor, not as a safety certification.

## Examples

`examples/adas_fca_monitor` is a self-contained forward-collision demo: a perception publisher, the monitor, and a planner that consumes the verdict, all over the stub on one box. `examples/lane_keep_cbf_control` shields an unsafe steering command. `examples/monitor_plus_control` wires the synthesize, monitor, and re-plan loop. Each carries its own manifests and a one-command `run.sh`.

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab. Dual licensed under MIT or Apache-2.0.