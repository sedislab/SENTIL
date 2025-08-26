# SENTIL benchmarks

One suite, one result shape, run the same way for every tool, so the numbers are directly comparable. The suite measures speed and memory against the baselines a runtime verification tool is judged by: RTAMT for STL, UPPAAL-SMC, PRISM, and Modest for statistical model checking.

## What is measured, and the fairness rule

Two questions get answered, and they are never mixed in one chart.

The full-signal question asks for the robustness at every sample of the trace, the whole signal. This is what RTAMT's offline evaluator computes, so the full-signal track is the like-for-like comparison against RTAMT: `Formula::robustness_signal` against RTAMT's offline `evaluate`, same formula, same trace.

The monitoring question asks for the robustness now, the value at the first sample, which is what an online monitor reports each step. SENTIL answers this by reading only the trace up to the formula's horizon, so the cost does not grow with the length of the trace behind it. This track is reported on its own, with its own ground truth, and is never quoted as the RTAMT speedup.

Every result record carries a `question` field set to `full_signal` or `monitoring`. The plotter groups strictly by that field. A reader can always tell which question a number answers.

## Layout

`deterministic/` holds the oracle: a fixed signal and formulas whose robustness is known by hand or closed form, so any monitor can be checked against the same ground truth before it is timed. `probabilistic/` holds stochastic models whose satisfaction probability is known. `scalability/` holds the length, depth, and bound sweeps. `runners/` holds one runner per tool, each emitting the shared JSON record. `results/` holds the committed JSON and the plots.

## Running

The deterministic, hardware-independent tier runs anywhere. The large sweeps belong on a compute node with a known, quiet CPU, not a shared login node, so the timings are stable and reproducible.