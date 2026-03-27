# Claims

Every performance and correctness claim SENTIL makes, with the command that reproduces it, the value to expect, the tolerance, and the tier it runs in. A claim marked confirmed is a test that passes or an artifact that regenerates. Where a number is bound to hardware we do not have, the expected value and the conditions are recorded rather than a measurement invented.

The tiers are CPU (runs anywhere, including in continuous integration), GPU (needs a device, skipped cleanly without one), tool-bound (needs a baseline tool we may not redistribute, skipped cleanly without it), and hardware-bound (tied to a specific machine).

The artifact-based claims below are checked automatically by `scripts/check_claims.py`, which reads the committed benchmark and experiment results and fails if a value falls outside its tolerance here. Run `python scripts/check_claims.py` directly, or get it as part of `make verify`. The exact claims (the deque equivalence, the interval coverage, the SPRT error rates, the oracle values) are guarded by the Rust test suite instead.

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
Expected: wilson(50, 100, 0.95) = [0.403831, 0.596169], cp(50, 100, 0.95) = [0.398321, 0.601679] (R's binom.test), z(0.95) = 1.959964.
Tolerance: 1e-6 for Wilson and z, 1e-3 for Clopper-Pearson.
Tier: CPU.

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

Expected: Flat in trace length, low single-digit microseconds for a bounded formula.
Tolerance: It does not grow with length.
Tier: CPU.

Breach is a dense-time tool, so the comparison is on the dense robustness value and on the monitoring question its `STL_Eval`/`CheckSpec` answers at time zero. The robustness matches bit for bit: on the length sweep formula both read -18.0736, and on the five oracle formulas SENTIL and Breach agree to the value.

On the monitoring question SENTIL answers in microseconds where Breach needs milliseconds, because Breach carries a fixed MATLAB and mex call overhead of a couple of milliseconds that dominates at these sizes while SENTIL reads only the formula's horizon.

| samples | SENTIL | Breach | speedup |
| --- | --- | --- | --- |
| 1,000 | 4.5 us | 4.86 ms | 1083x |
| 10,000 | 4.5 us | 2.11 ms | 473x |
| 100,000 | 4.5 us | 2.49 ms | 549x |
| 1,000,000 | 4.6 us | 6.97 ms | 1514x |

Command: `cargo run --release -p sentil-benchmarks --bin sentil_runner -- scalability` and the Breach runner `matlab -batch "breach_runner('scalability')"` with Breach on the path, then `python benchmarks/runners/plot.py`.
Expected: Identical robustness and SENTIL a few hundred to a few thousand times faster.
Tolerance: 0 tolerance for the robustness.
Tier: CPU.
Artifact: `benchmarks/results/sentil_scalability.jsonl`, `breach_scalability.jsonl`, `breach_deterministic.jsonl`.

Computing the whole dense robustness signal, SENTIL's cost grows linearly with length and runs about 7x to 12x the discrete full-signal cost, the price of the segment interpolation dense time needs.

| samples | dense full signal | discrete full signal |
| --- | --- | --- |
| 1,000 | 0.31 ms | 0.037 ms |
| 100,000 | 44.2 ms | 3.64 ms |
| 1,000,000 | 389 ms | 37.06 ms |

Command: `cargo run --release -p sentil-benchmarks --bin sentil_runner -- dense`.
Expected: a single-digit multiple of the discrete cost.
Tolerance: 
Tier: CPU.
Artifact: `benchmarks/results/sentil_dense.jsonl`.

## Streaming

Per-sample latency on the online monitor, the nested formula `always[0, 100](eventually[0, 10](x > 5))` driven one sample at a time through `StreamMonitor::update_packed`.
Command: `cargo run --release -p sentil-benchmarks --bin sentil_runner -- streaming`.
Measured on one EPYC core over a million samples: median 81 ns, p99 120 ns, mean 87 ns. Each update is timed on its own, so two clock reads of a few tens of nanoseconds are folded into every figure and the true per-sample cost is lower. The tail sits within about one and a half times the median, and the monitor sustains over eleven million updates per second, far above the ten kilohertz target. Tolerance: report the measured number, do not target one; the exact figure is hardware-bound. Tier: CPU.

Memory is proportional to the largest temporal window, not the trace length, so an arbitrarily long stream holds steady resident memory for a given formula.

## Cross-language call overhead

The same streaming monitor driven one sample at a time from each binding, on the nested formula above, every binding reading the identical robustness -17.99212. The per-sample time is the cost of one call across the language boundary into the core plus the update itself.

| binding | per-sample update |
| --- | --- |
| C | 74 ns |
| Rust (core, no FFI) | 112 ns |
| Julia | 114 ns |
| C++ | 134 ns |
| Python | 521 ns |
| Java | 680 ns |
| MATLAB | 6.49 us |

Command: run each language's streaming runner, for instance `python benchmarks/runners/sentil_runner.py streaming`, then `python benchmarks/runners/plot.py` for the cross-language figure.
Expected: every binding under a microsecond except MATLAB's interpreter path.
Tolerance: The claim is that the call overhead is indicated roughly by these scales.
Tier: CPU.
Artifact: `benchmarks/results/sentil_streaming_*.jsonl`.

## Synthesis

Open-loop trajectory synthesis finds an input sequence that satisfies the spec on a linear model, and the receding-horizon controller plans one online within a hard step deadline.

| case | backend | robustness | time |
| --- | --- | --- | --- |
| hold, offline | gradient | 0.50 | 1.72 ms |
| reach, offline | gradient | 4.00 | 1.26 ms |
| bounded input, offline | gradient | 0.40 | 2.90 ms |
| hold, offline | CMA-ES | 0.50 | 5.87 ms |
| integrator hold, online | gradient | 0.50 | 0.099 ms/step |

The online controller ran 200 steps against a 5 ms deadline with a p99 of 0.112 ms and no misses, so it plans each input in about a tenth of a millisecond with room to spare.
Command: `cargo run --release -p sentil-benchmarks --bin sentil_synth_runner`.
Expected: every offline case reaches a positive robustness (the spec holds on the model), the online controller misses no deadline.
Tolerance: robustness within 1e-3 of the recorded value with zero deadline misses.
Tier: CPU.
Artifact: `benchmarks/results/sentil_synth.jsonl`.

## Confidence intervals and sequential testing

Coverage on synthetic ground truth. Over 4000 batches of 100 Bernoulli draws at a known p of 0.3, counting how often the 95 percent interval contains p, the Wilson interval covers within 0.03 of its nominal 0.95 and the Clopper-Pearson interval covers at least 0.94, the conservative behavior it is built for. The seed is fixed, so the run is deterministic.
Command: `cargo test --offline -p sentil --features statistical wilson_and_clopper_pearson_cover_at_their_nominal_rate`.
Expected: Wilson coverage within 0.03 of 0.95, Clopper-Pearson at least 0.94. Tolerance: as stated in the assertion. Tier: CPU, every commit.

SPRT error rates stay under their nominal bounds. Wald's test with p0 = 0.3, p1 = 0.7, and alpha = beta = 0.05, run 400 times against a process at p = 0.2 (deep in H0) and again at p = 0.8 (deep in H1), accepts the wrong hypothesis at most 10 percent of the time on each side, inside the nominal error the test is configured for.
Command: `cargo test --offline -p sentil --features statistical the_error_rates_stay_within_the_bounds`.
Expected: Type I rate at most 0.1, Type II rate at most 0.1. Tolerance: as stated. Tier: CPU, every commit.

## Statistical model checking, against UPPAAL-SMC, PRISM, Modest

These are probabilistic model checkers, a different paradigm from SENTIL's trace and system monitoring, so a fair comparison needs a shared model every tool simulates. Two carry all four tools. The Barkai-Leibler circadian CTMC asks whether the activator reaches 100 within 20 time units, a rare-ish event near 0.04. The tandem queue asks whether either queue fills within 50 time units, a common event near 0.72, which exercises the opposite end of the variance range. Both are estimated at 10,000 samples, except by UPPAAL, which chooses its own count. Two further models, the biodiesel reactor and the powertrain controller, are continuous-state discrete-time recurrences, which SENTIL and PRISM do not express; Modest and UPPAAL both run them, and their agreement cross-checks the ports.

PRISM runs both cleanly: about 0.041 on the circadian model (0.0393 to 0.0438 across runs, half-width about 0.005) in about 27 seconds, and 0.726 on the tandem queue in about 5 seconds. SENTIL simulates the identical CTMCs with Gillespie's direct method and estimates the same probabilities by direct Monte Carlo, 0.0378 in about 0.8 seconds and 0.7151 in about 0.3 seconds, roughly 30 and 17 times faster with agreeing estimates. Its `tandem.nm` needed one change to parse at all: the model named a constant `C`, which PRISM reserves for the cumulative reward operator.
Command: `make bench-smc` and `PRISM=<prism> make bench-prism`.
Expected: both near 0.04 within the confidence interval, SENTIL more than an order of magnitude faster. Tolerance: the estimates agree within the intervals; the speedup is machine and load dependent. Tier: CPU and tool-bound. Artifact: `benchmarks/results/sentil_smc.jsonl`, `prism_smc.jsonl`.

Modest runs it once the reference models are ported to the v3.1.301 grammar, which `benchmarks/baselines/modest` carries. It estimates 0.038 (half-width about 0.004 at 95 percent) in about 73 seconds on the same machine, agreeing with SENTIL and PRISM. The toolset may not be redistributed, so it is never vendored here and never installed in continuous integration, and the runner skips without it. It estimates 0.720 on the tandem queue in about 10 seconds, 0.193 on the biodiesel reactor, and 0.501 on the powertrain controller.
Command: `MODEST=<modest> bash benchmarks/runners/modest_runner.sh benchmarks/baselines/modest/circadian.modest`, or `make bench-modest` for all four.
Expected: near 0.04 within the interval, SENTIL more than an order of magnitude faster. Tolerance: the estimates agree within the intervals; Modest's wall time ranged from about 43 to 73 seconds across runs on a shared machine, so the ratio is an order-of-magnitude statement rather than a fixed figure. Tier: CPU and tool-bound. Artifact: `benchmarks/results/modest_smc.jsonl`.

UPPAAL-SMC runs it once the model is rebuilt for the fragment it samples, which `benchmarks/baselines/uppaal` carries. It estimates 0.034 on the circadian model in about 19 seconds and 0.719 on the tandem queue in about 21 seconds, agreeing with the other three. Two faults had to go. The reference model raced the seven reactions with probability weights on ordinary edges, which UPPAAL ignores, selecting each edge with equal chance instead, so the rebuilt model races them through a branch point; a two-edge test with weights 9 and 1 returns a 1 to 1 split on ordinary edges and 9 to 1 through a branch point. It also carried a peak-detection sub-automaton whose general expression guards sit outside the samplable fragment, which is what produced the earlier degenerate verdict. It also runs the two continuous-state models, as stochastic hybrid automata that hold the reactor state in rate-zero clocks and step the Euler recurrence one interval at a time, estimating 0.193 on the biodiesel reactor and 0.497 on the powertrain controller, both agreeing with Modest. UPPAAL picks its own run count from the requested half-width rather than taking a fixed sample size, so it used about 9000 runs on the circadian model and 48000 on the higher-variance tandem queue, against the fixed 10,000 of the others; read its times with that in mind.
Command: `VERIFYTA=<verifyta> make bench-uppaal`.
Expected: near 0.04 within the interval, SENTIL more than an order of magnitude faster. Tolerance: the estimates agree within the intervals; the run count is UPPAAL's own and the wall time is machine and load dependent. Tier: CPU and tool-bound. Artifact: `benchmarks/results/uppaal_smc.jsonl`.

## GPU acceleration

The Monte Carlo counting and the rare-event splitting run on a WebGPU device, with a clean fall back to the CPU when none is present. The device path is validated on an NVIDIA A40. Its tests are gated behind the `gpu` feature, skip cleanly with no device, and must run single-threaded: each builds its own device, and the driver does not survive several created at once.
Command: `cargo test --offline --no-default-features --features synthesis-gpu -- --ignored --test-threads=1` on a GPU node.
Expected: all 18 device tests pass. The on-device results match the CPU and the closed form to single precision: the satisfaction count tracks the normal CDF, the temporal robustness sign matches the CPU monitor across every operator, the splitter recovers the analytic crossing probability, and the soft robustness agrees with the CPU within an f32 tolerance. Tier: GPU.

Throughput. On the A40 the Monte Carlo kernel sustains about 829 million realizations per second for `x > 0` under additive standard-normal noise, against about 7.9 million per second on one EPYC core running the full lift-and-score path, a speedup near 105x over a single core. The CPU path scales across cores, so one device sits in the range of many cores for this kernel, and a heavier temporal formula gives the device more work per realization and widens the gap.
Command: `cargo test --release --offline --no-default-features --features synthesis-gpu -- --ignored gpu_smc_throughput --test-threads=1 --nocapture`.
Expected: a GPU-over-one-core speedup near two orders of magnitude; the exact figure is hardware-bound. Tier: GPU.

Rare events. The adaptive multilevel splitting resolves satisfaction probabilities that plain Monte Carlo cannot reach at the same sample budget. On the A40 it recovers a three-sigma crossing probability of about 0.0027 within 25 percent of the analytic value over eight seeds, and agrees within a factor of two with the CPU last-particle splitter, a different but equally valid estimator, so a rare-event probability differs by scheme as well as by seed. Splitting is the mechanism that carries the resolvable probability into the 1e-7 to 1e-9 range a flat Monte Carlo run of feasible size never reaches. Tier: GPU.

Particle-count convergence. On a Gaussian random walk, the continuous-score regime the splitter is built for, the estimate tightens toward a Monte Carlo reference as the particle population grows. For a moderate event, truth 3.3e-2, the mean relative error over five seeds falls from 0.16 at 100 particles to 0.01 at 8000; for a rare event, truth 6.0e-5, which a thousand-sample plain run never sees, it falls from 0.38 to 0.07. On a discrete score with heavy ties the level selection biases the estimate, so this is measured where the score is continuous.
Command: `cargo run --release -p sentil-benchmarks --bin sentil_particle_runner`.
Expected: the mean relative error decreases with particle count and sits under about 0.15 at 8000 particles for both events. Tolerance: the reference is an 8e6-sample Monte Carlo run in the same runner; the estimate has seed and scheme variance. Tier: CPU. Artifact: `benchmarks/results/sentil_particles.jsonl`.

## Embedded deployment, Raspberry Pi 4

SENTIL runs as the safety monitor in an autonomous-driving stack on a Raspberry Pi 4 Model B (Broadcom BCM2711, quad-core Cortex-A72 at 1.5 GHz, 4 GB LPDDR4, 64-bit Raspberry Pi OS), under 4 W. One cycle ingests the latest observation and evaluates 60 safety specifications, returning a robustness score for each, at 85 Hz over a 120-minute drive of 612,000 cycles. The streaming monitor's flat per-sample cost is what carries the whole workload inside the deadline.

These figures are recorded from the reference deployment on that board and are hardware-bound; reproduce them on a Pi 4, not on a workstation.

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
| Memory growth over two hours | 0 MB |

Command: `experiments/embedded_deployment/run_deployment.sh --duration 120`, cross-compiled with `cross build --release --target aarch64-unknown-linux-gnu -p sentil-embedded-deployment` and run on the board.
Expected (Pi 4): mean 9.57, median 9.44, p95 10.62, p99 10.71, max 11.09 ms; deadline 11.76 ms; 0 violations of 612,000; steady 12.3 MB; 0 MB growth. Tolerance: latency within about 15 percent, zero deadline violations, memory within about 20 percent. Tier: hardware-bound (Pi 4).

On a workstation the harness is a regression guard rather than a reproduction: the per-cycle cost is microseconds, not milliseconds, but it stays flat as the drive lengthens and the resident memory holds near the reference with a small bounded growth that does not scale with the run, so the monitor leaks nothing over the drive. The reference also ran ten probabilistic specifications through the statistical layer; those use the sampling path, not the streaming one, and their cost is part of the recorded numbers rather than the regenerated figure.

The ARM comparison: RTAMT on the same Pi takes about 47 ms per cycle, four times over the deadline, and Breach does not run on ARM at all. Tier: hardware-bound (Pi 4) and tool-bound.

## Autonomous driving, CARLA

SENTIL monitors a vehicle driving through CARLA against a compound specification: lane keeping within 0.3 m, 5 m clearance from other agents, an urban speed limit, and a probabilistic conjunct `P>=0.99(always[0,10] (no collision))` that weighs a ten-second collision-free lookahead against the uncertainty in where pedestrians go. The trace is a 300-second, 6000-frame drive on the Town10HD map under the CARLA Traffic Manager, recorded from a live server and monitored offline, so the verdicts reproduce with no GPU.

The latency is CPU work on the monitoring machine, not the GPU node that runs the simulator.

| Metric | Value |
| --- | --- |
| Deterministic streaming, per sample | about 0.54 us |
| Deterministic streaming, sustained | about 1.8 M samples/s |
| Probabilistic check, median per frame | about 0.65 ms |
| Probabilistic check, 99th percentile | about 0.89 ms |
| Closed-loop deadline | 2 ms |

Command: record with `experiments/carla_driving/record_drive.py` against a CARLA 0.9.15 server, then `python experiments/carla_driving/monitor_drive.py --trace experiments/carla_driving/results/drive.json`.
Reference (STORM paper, Apollo on an A100 node): median 0.64 ms, 99th 1.83 ms, end to end 2.1 ms per frame; an RTAMT monitor on the same workload about 47 ms. Tolerance: the deterministic streaming stays sub-microsecond and the probabilistic check stays inside the 2 ms deadline at the median and the tail; absolute figures are machine-dependent. Tier: CPU (the recording step is GPU-bound and not part of the regenerated number).

The probabilistic conjunct is what the deterministic checks cannot do. At the pedestrian encounter near t = 146 s the deterministic clearance still reads 5.7 m, above the 5 m bound, while the collision-free probability over the lookahead falls to essentially zero, because the pedestrian's predicted path under its uncertainty meets the car's recorded path inside ten seconds. The probabilistic verdict holds near 1.0 across the rest of the drive and drops only at the few real encounters. This mirrors the STORM paper's intersection event, where deterministic bounds held and the collision-free probability fell to 0.94. Tier: CPU.

## Medical device, artificial-pancreas glucose control

SENTIL checks a closed-loop insulin controller on a type-1 patient simulated with the FDA-accepted UVA/Padova model (Dalla Man 2007, S2013 risk-based utilization, average-adult parameters) over a 24-hour day of three meals, against clinical safety specifications, both on the true glucose and probabilistically under the continuous-glucose-monitor noise. The model holds fasting glucose at about 120 mg/dL and gives a bolused meal a peak near 180 that recovers, matching the reference. Two controllers run: one that skips the lunch bolus, a real and dangerous lapse, and a tuned one that doses every meal.

The missed-bolus controller violates the euglycemia band (robustness -105 mg/dL, the excursion reported as the interval [809, 1424] minutes as glucose runs from lunch to the end of the day and peaks near 285), while the tuned controller holds it (robustness +8). Both stay clear of severe hypoglycemia, and the probabilistic hypoglycemia-safety check `P>=0.95(always (glucose > 70))` holds for both at probability about 1.0, since neither goes low. The point is that the deterministic euglycemia verdict separates the two controllers while the hypoglycemia risk stays low under sensor noise for both.
Command: `python experiments/glucose_control/glucose_control.py`.
Expected: euglycemia violated for the missed-bolus controller (about -105) and satisfied for the tuned one (about +8), the missed-bolus excursion near [809, 1424] min, both hypoglycemia probabilities near 1.0. Tolerance: missed-bolus robustness within 6 mg/dL, tuned within 2.5, identical verdicts, probability within 0.01. Tier: CPU. Artifact: `experiments/glucose_control/results/glucose.json`.

## Gene-regulatory network, circadian oscillation

SENTIL verifies that a Barkai-Leibler circadian gene network keeps oscillating, a temporal property no single threshold captures, over a reference ensemble of 100 stochastic realizations of the activator protein sampled hourly for 270 hours. The oscillation is expressed as two recurrences, a peak above 3000 within every 24-hour window and a trough below 2000 within every window, and the network must satisfy both.

On the ensemble mean the peak recurrence holds with robustness about +2530 and the trough recurrence with about +1040, the measured period is 23.8 hours, and the amplitude runs from 403 to 6218. All 100 realizations satisfy the joint property, so the empirical satisfaction probability is 1.0, and a probabilistic check that lifts each reading by a measurement error holds at probability 1.0 because the peak margin dwarfs the noise.
Command: `python experiments/circadian_gene_network/circadian_gene_network.py`.
Expected: both recurrences satisfied on the mean (peaks about +2530, troughs about +1040), period near 24 hours, all 100 realizations oscillating. Tolerance: robustness within a few percent, period within about 1 hour, verdicts identical, the ensemble fraction exact on the shipped traces. Tier: CPU. Artifact: `experiments/circadian_gene_network/results/circadian.json`.

## The Lean proof

The monotonic-deque sliding-window theorem is machine-checked, for both the minimum (always, historically) and the maximum (eventually, once) cases, over a decidable linear order.
Command: `cd proofs && lake build`.
Expected: builds clean, no `sorry`, no `admit`, axioms exactly `[propext, Quot.sound]`. Tolerance: exact. Tier: CPU.

Two caveats remain on the proof. The executable `#eval` checks use a separate array implementation as corroboration, and the link from the proof to the Rust code is by inspection, not mechanized. The back eviction drops a candidate whose value is greater than or equal to the incoming one, ties included, which matches the Rust; any prose that says strictly greater is reconciled to this.

## Build

A from-scratch release build of the core finishes well under a minute, with fat link-time optimization and a single codegen unit.
Command: `cargo build --release --offline --no-default-features`.
Expected: under a minute on the reference node. Tolerance: machine-dependent. Tier: CPU.