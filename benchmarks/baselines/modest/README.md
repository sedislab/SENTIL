# Modest SMC baseline models

We test four models on the Modest Toolset. Each model gives a statistical model checking baseline for the SENTIL statistical layer. Modest, PRISM and SENTIL all check the circadian network with the same property. You can compare their numbers directly.

## The models

| File | What it models | Ported from | Change made |
| --- | --- | --- | --- |
| `circadian.modest` | Barkai-Leibler gene network |
| `tandem_queue.modest` | two queues in series |
| `biodiesel.modest` | a reactor with a heater that can fail |
| `powertrain.modest` | air-fuel-ratio control |

## Running them

Get the Modest Toolset from `modestchecker.net`. Then install it.

Set `MODEST` to the launcher. Then run one model:

```
MODEST=<path>/modest bash benchmarks/runners/modest_runner.sh benchmarks/baselines/modest/circadian.modest
```

The runner does 10,000 simulation runs with a fixed seed. It writes one JSON record to standard output.

To run all four models, use `make bench-modest`. The command writes `benchmarks/results/modest_smc.jsonl`.