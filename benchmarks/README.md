# SENTIL benchmarks

One suite, one record shape, run the same way for every tool, against the baselines a runtime verification tool is judged by: RTAMT for STL, and UPPAAL-SMC, PRISM, and Modest for statistical model checking.

## Two questions, never mixed

The full-signal question asks for the robustness at every sample. This is what RTAMT's offline evaluator computes, so the full-signal track is the like-for-like comparison: `Formula::robustness_signal` against RTAMT's `evaluate`, same formula, same trace.

The monitoring question asks for the robustness at the first sample, the value an online monitor reports each step. SENTIL reads only the trace up to the formula's horizon, so the cost does not grow with the history behind it. This track is reported on its own and is never quoted as the RTAMT speedup.

Every record carries a `question` field, `full_signal` or `monitoring`. The plotter groups strictly by it.

## Layout

`deterministic/` is the oracle, fixed signals and formulas with known robustness. `probabilistic/` holds models with known satisfaction probability. `scalability/` holds the length, depth, and bound sweeps. `runners/` holds one runner per tool, each emitting the shared record. `results/` holds the committed JSON and the plots.

## Running

The deterministic tier runs anywhere. The large sweeps belong on a quiet compute node, not a shared login node, so the timings are stable.

## Statistical model checking

`sentil_smc_runner` estimates a model's satisfaction probability and records it with its confidence interval, the closed-form truth where there is one, and the sampling throughput. Two suites share one estimator. The accuracy suite runs every known-probability model from `probabilistic/` and scores the estimate against the truth, so a run checks correctness, not just speed. The throughput suite runs cheap formulas at high sample counts, where the cost is the noise draws and the per-realization score rather than the formula, so the realization-step rate is the number to compare.

Run the CPU path with `cargo run --release --bin sentil_smc_runner accuracy` or `... throughput <samples>`. For the GPU path, build with `--features gpu` on a machine that has a compatible device and run the same commands; the runner labels each record `cpu` or `gpu` by what actually ran, so a feature-on run on a device-less node still reports the truth.

The committed numbers were measured on a 128-core AMD EPYC 7763 (`sentil_smc_cpu.jsonl`) and an NVIDIA A40 (`sentil_smc_gpu.jsonl`), and they say something worth stating plainly: for flat Monte Carlo on this hardware the parallel CPU path is the faster one. It sustains 800 to 900 million realization-steps per second across the cores, while the A40 peaks near 290 million around ten million samples and falls off above that, because each estimate pays the device setup afresh and the kernels are sized for the rare-event splitter rather than a flat sweep. The GPU earns its place on the rare-event path, where it resolves probabilities far below what a flat Monte Carlo run reaches at all, not on raw sampling rate against a high-core-count CPU.

## Baselines

RTAMT is the STL baseline and runs through `rtamt_runner.py`. The statistical baselines are harder to pin down honestly. The Modest toolset is bundled with the case-study models, but those models were written against an older Modest grammar and do not parse under the bundled v3.1.301, so no Modest numbers are reproduced here; the UPPAAL-SMC runs in the case studies failed on four of five models. Where a baseline does not reproduce cleanly its status is recorded rather than a number invented.