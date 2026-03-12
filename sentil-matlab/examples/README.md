# Examples

A small set of exmaples to show how to use the tool. Build the binding once, then run an example from the `sentil-matlab` directory:

```matlab
build_sentil
addpath(pwd, fullfile(pwd, 'examples'))
offline_monitoring
```

`offline_monitoring` evaluates a formula over a recorded trace in discrete and dense time. `online_streaming` folds one sample at a time and reports the first violation. `probabilistic` lifts a noisy sensor and estimates satisfaction with a confidence interval. `synthesis` finds a control input that satisfies a spec, then shields a nominal input back into its bounds.

The Simulink counterpart is `run_fda_insulin_benchmark`, which drives the `SENTIL Monitor` block from `blocks/create_sentil_library` over the UVA/Padova artificial-pancreas model.