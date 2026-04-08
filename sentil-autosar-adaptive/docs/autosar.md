---
id: autosar
title: AUTOSAR Adaptive
sidebar_position: 5
---

# AUTOSAR Adaptive

`sentil-autosar-adaptive` exposes SENTIL on the AUTOSAR Adaptive Platform over ara::com. A `sentil_monitor` application evaluates Signal Temporal Logic and probabilistic STL and offers a verdict; a `sentil_control` application synthesizes or shields a command. The compiled SENTIL core ships inside each app, so the ECU needs no Rust toolchain. The package README is the full reference; this page is the integration overview.

## Running without a vendor platform

The package ships an open-source ara::com over vsomeip, so the apps and the examples run on a plain Linux box with no Adaptive Platform license:

```bash
cmake -B build -G Ninja -DSENTIL_AP_VENDOR=stub \
  -DCMAKE_TOOLCHAIN_FILE=cmake/toolchains/linux-x86_64-stub.cmake -DSENTIL_ROOT=/path/to/sentil
ninja -C build
ctest --test-dir build
```

A vendor AP is a toolchain swap: `-DSENTIL_AP_VENDOR=vector|eb|apex` with the vendor toolchain points `FindAraCom` at the vendor binding and runs the vendor generator on the same `model/*.arxml`.

## The deployment

Three ARXML interfaces in `model/` (signal, verdict, control) are separate from their deployment in `manifest/`, the AUTOSAR methodology split. The manifest trio per app is the machine manifest (unicast, service discovery, function groups), the execution manifest (process binding, function-group states, scheduling), and the service-instance manifest (the SOME/IP service, instance, event, and event-group ids), rendered into the vsomeip configs. Set the SENTIL thread count to one on a hard-real-time target for a tighter worst-case execution time.

## Health and safety

The monitor is a Platform Health Management Supervised Entity and it reports a checkpoint each cycle and raises a Diagnostic Event Management event on a sustained violation, so a falsified property or a blown deadline reaches the platform's health machinery. SENTIL is a verdict source and a synthesis engine, not the safety case. The core is a QM element, a downstream safety monitor acts on the verdict, and the integrator owns the ISO 26262 argument. The Lean-proved monotonic deque is formal-methods evidence for the streaming monitor, not a certification.