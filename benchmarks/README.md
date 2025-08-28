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