# Embedded deployment: per-cycle latency on a Raspberry Pi 4

This experiment measures what it costs to run SENTIL as the safety monitor in an autonomous-driving stack on a Raspberry Pi 4. One cycle ingests the latest observation and evaluates every safety specification, returning a robustness score for each. The question is whether 60 specifications fit inside the 1/85 Hz budget on a 4 W board, cycle after cycle, for hours.

## What it runs

The workload is 60 streaming safety specifications over nine signals reduced from the sensor stack: speed, following distance, time to collision, acceleration, jerk, lane offset, sensor-fusion confidence, collision probability, and yaw rate. The specifications cover plain bounds (speed and acceleration limits, a minimum following distance), nested response requirements (a hazard must be answered within a short window), and longer-horizon guarantees, with nesting depth from 1 to 3. They run on the streaming monitor, the flat per-sample engine the latency claim rests on.

The reference deployment additionally ran ten probabilistic specifications through the statistical layer. Those use the sampling path rather than the streaming one, so they sit outside this harness; their cost is part of the recorded Pi 4 numbers below, not the regenerated figure.

The signals come from a seeded generator that walks each value slowly inside its safe band, so the sliding windows behave like a real drive. The `violation_cycles` field counts cycles where some specification's running robustness was momentarily negative; it is a provisional per-sample count, not the confirmed violation intervals, and it depends on the synthetic drive rather than reproducing the reference drive's findings.

## Running it

```
experiments/embedded_deployment/run_deployment.sh --duration 120
```

The reference drive is 120 minutes at 85 Hz, which is 612,000 cycles. `--cycles N` sets the count directly, `--rate` the input rate, `--seed` the drive, and `--hardware` the label recorded in the result. The result lands in `results/embedded.json`.

Run it on a Raspberry Pi 4 to reproduce the published latency. On a workstation it is a regression guard: the per-cycle cost is microseconds rather than the Pi's milliseconds, but it must stay flat as the drive lengthens, which is the property that makes the deadline hold on the board. Cross-compile for the Pi with `cross build --release --target aarch64-unknown-linux-gnu -p sentil-embedded-deployment` and copy the binary across, or build it natively on the Pi.

## The reference Pi 4 numbers

These are recorded from the reference deployment on a Raspberry Pi 4 Model B (Broadcom BCM2711, quad-core Cortex-A72 at 1.5 GHz, 4 GB LPDDR4, 64-bit Raspberry Pi OS), under 4 W, over a 120-minute drive of 612,000 cycles. They are hardware-bound; reproduce them on that board, not on a workstation.

| Metric | Value |
| --- | --- |
| Mean latency | 9.57 ms |
| Median latency | 9.44 ms |
| 95th percentile | 10.62 ms |
| 99th percentile | 10.71 ms |
| Maximum observed | 11.09 ms |
| Real-time deadline | 11.76 ms (1/85 Hz) |
| Deadline violations | 0 of 612,000 |
| Steady-state memory | 12.3 MB |
| Memory growth over 2 hours | 0 MB |
| RTAMT on the same Pi | 47 ms, four times over the deadline |
| Breach on ARM | does not run |

The thermal envelope was about 55 C in a 25 C ambient, well under the 85 C throttle threshold, and a native build of the full workspace took about 12 minutes on the board.

The workstation regression-guard run holds well under the deadline with a flat per-cycle cost, a steady memory near the reference, and a small bounded growth that does not scale with the drive length, so the streaming monitor leaks nothing as the run goes on. See `docs/CLAIMS.md` for the tolerances.