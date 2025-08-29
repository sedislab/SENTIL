# Claims

Every performance and correctness claim SENTIL makes, with the command that reproduces it, the value to expect, the tolerance, and the tier it runs in. A claim marked confirmed is a test that passes or an artifact that regenerates. Where a number is bound to hardware we do not have, the expected value and the conditions are recorded rather than a measurement invented.

The tiers are CPU (runs anywhere, including in continuous integration), GPU (needs a device, skipped cleanly without one), and hardware-bound (tied to a specific machine).

## How the speed numbers are read

1. Prefix scoring equals full scoring. `Formula::robustness` reads only the dependency prefix, and its value at the first sample matches scoring the whole trace, to the bit, over random formulas and traces.
Command: `cargo test --offline -p sentil prefix_robustness_equals_full_at_index_zero` (and `PROPTEST_CASES=20000` for the deep run).
Expected: every case equal by `to_bits`.
Tolerance: exact.
Tier: CPU.

Deque equals naive. The monotonic-deque sliding window matches the exhaustive scan on bounded and unbounded windows, both extrema. This is the equivalence the Lean proof formalizes.
Command: `cargo test --offline -p sentil deque_equals_naive`.
Expected: all equal.
Tolerance: exact.
Tier: CPU.

Monte Carlo reuse changes no count. The reused per-worker trace buffer gives the same satisfaction count, bit for bit, as a fresh lift scored over the whole signal, including with several registered noise signals where the draw order matters.
Command: `cargo test --offline -p sentil count_matches_the_fresh_lift_full_robustness_baseline`.
Expected: identical count.
Tolerance: exact.
Tier: CPU.

Oracle robustness. SENTIL reproduces the known robustness of every canonical formula on the fixed oracle trace.
Command: `cargo test --offline -p sentil-benchmarks sentil_reproduces_every_oracle_value`.
Expected: phi1 = -7.622064772118447, phi2 = 4.993604045622577, phi3 = 1.0, phi4 = 1.0, phi5 = -1.0.
Tolerance: exact.
Tier: CPU.

Wilson and Clopper-Pearson reference values. The interval functions return the published values on a known input.
Command: `cargo test --offline -p sentil --features statistical confidence`.
Expected: wilson(50, 100, 0.95) = [0.403831, 0.596169], cp(50, 100, 0.95) = [0.39828, 0.60172] (R's binom.test), z(0.95) = 1.95996. Tolerance: 1e-6 for Wilson and z, 1e-3 for Clopper-Pearson. Tier: CPU.

## Speed: full-signal track, against RTAMT

Discrete STL, whole robustness signal, formula `always[0, 100](eventually[0, 10](x > 5))`, same node, same trace, identical robustness.
Command: `cargo run --release -p sentil-benchmarks --bin sentil_runner -- scalability` and `python benchmarks/runners/rtamt_runner.py scalability`, then `python benchmarks/runners/plot.py`.

| samples | SENTIL | RTAMT | speedup |
| --- | --- | --- | --- |
| 1,000 | 0.037 ms | 6.10 ms | 163x |
| 10,000 | 0.399 ms | 51.4 ms | 129x |
| 100,000 | 3.64 ms | 514 ms | 141x |
| 1,000,000 | 37.06 ms | 5381 ms | 145x |

Expected: SENTIL faster by roughly two orders of magnitude across the range.
Tolerance: At least 100x speedup.
Tier: CPU.
Artifact: `benchmarks/results/`.

Per formula at 2001 samples, the full-signal speedup over RTAMT ranges from about 100x (phi1) to about 220x (phi5), recorded in `benchmarks/results/sentil_deterministic.jsonl` and `rtamt_deterministic.jsonl`.

## Speed: monitoring track

The monitoring value costs the same whatever the length of trace behind it, because only the samples within the formula's horizon are read. For the bounded formula above, the time per evaluation is flat near 0.005 ms from one thousand to ten million samples, while the full-signal cost grows with length.

| samples | monitoring | full signal |
| --- | --- | --- |
| 1,000 | 0.0045 ms | 0.037 ms |
| 100,000 | 0.0045 ms | 3.64 ms |
| 10,000,000 | 0.0048 ms | 391.8 ms |

Expected: flat in trace length, low single-digit microseconds for a bounded formula. Tolerance: within a small factor across machines; the defining property is that it does not grow with length. Tier: CPU. This is not quoted as the RTAMT speedup, since RTAMT answers the full-signal question.

## Streaming

Per-sample latency on the online monitor, the nested formula `always[0, 100](eventually[0, 10](x > 5))` driven one sample at a time through `StreamMonitor::update_packed`.
Command: `cargo run --release -p sentil-benchmarks --bin sentil_runner -- streaming`.
Measured on one EPYC core over a million samples: median 81 ns, p99 120 ns, mean 87 ns. Each update is timed on its own, so two clock reads of a few tens of nanoseconds are folded into every figure and the true per-sample cost is lower. The tail sits within about one and a half times the median, and the monitor sustains over eleven million updates per second, far above the ten kilohertz target. Tolerance: report the measured number, do not target one; the exact figure is hardware-bound. Tier: CPU.

Memory is proportional to the largest temporal window, not the trace length, so an arbitrarily long stream holds steady resident memory for a given formula.

## Statistical model checking, against UPPAAL-SMC, PRISM, Modest

On the reference model set, `verifyta` segfaults on all five models under the container it was run in, so no UPPAAL-SMC timing is available to compare against; SENTIL completes the same checks. A speedup is quoted only against a model UPPAAL actually finishes. PRISM and Modest have models and scripts but no committed run on this hardware; their expected values are recorded from the reference project rather than measured here.
Tier: hardware-bound and tool-bound. State the situation plainly rather than a number the runs do not support.

## GPU acceleration

The Monte Carlo counting and the rare-event splitting run on a WebGPU device, with a clean fall back to the CPU when none is present. The device path is validated on an NVIDIA A40. Its tests are gated behind the `gpu` feature, skip cleanly with no device, and must run single-threaded: each builds its own device, and the driver does not survive several created at once.
Command: `cargo test --offline --no-default-features --features synthesis-gpu -- --ignored --test-threads=1` on a GPU node.
Expected: all 18 device tests pass. The on-device results match the CPU and the closed form to single precision: the satisfaction count tracks the normal CDF, the temporal robustness sign matches the CPU monitor across every operator, the splitter recovers the analytic crossing probability, and the soft robustness agrees with the CPU within an f32 tolerance. Tier: GPU.

Throughput. On the A40 the Monte Carlo kernel sustains about 829 million realizations per second for `x > 0` under additive standard-normal noise, against about 7.9 million per second on one EPYC core running the full lift-and-score path, a speedup near 105x over a single core. The CPU path scales across cores, so one device sits in the range of many cores for this kernel, and a heavier temporal formula gives the device more work per realization and widens the gap.
Command: `cargo test --release --offline --no-default-features --features synthesis-gpu -- --ignored gpu_smc_throughput --test-threads=1 --nocapture`.
Expected: a GPU-over-one-core speedup near two orders of magnitude; the exact figure is hardware-bound. Tier: GPU.

Rare events. The adaptive multilevel splitting resolves satisfaction probabilities that plain Monte Carlo cannot reach at the same sample budget. On the A40 it recovers a three-sigma crossing probability of about 0.0027 within 25 percent of the analytic value over eight seeds, and agrees within a factor of two with the CPU last-particle splitter, a different but equally valid estimator, so a rare-event probability differs by scheme as well as by seed. Splitting is the mechanism that carries the resolvable probability into the 1e-7 to 1e-9 range a flat Monte Carlo run of feasible size never reaches. Tier: GPU.

## The Lean proof

The monotonic-deque sliding-window theorem is machine-checked, for both the minimum (always, historically) and the maximum (eventually, once) cases, over a decidable linear order.
Command: `cd proofs && lake build`.
Expected: builds clean, no `sorry`, no `admit`, axioms exactly `[propext, Quot.sound]`. Tolerance: exact. Tier: CPU.

Two caveats remain on the proof. The executable `#eval` checks use a separate array implementation as corroboration, and the link from the proof to the Rust code is by inspection, not mechanized. The back eviction drops a candidate whose value is greater than or equal to the incoming one, ties included, which matches the Rust; any prose that says strictly greater is reconciled to this.

## Build

A from-scratch release build of the core finishes well under a minute, with fat link-time optimization and a single codegen unit.
Command: `cargo build --release --offline --no-default-features`.
Expected: under a minute on the reference node. Tolerance: machine-dependent. Tier: CPU.