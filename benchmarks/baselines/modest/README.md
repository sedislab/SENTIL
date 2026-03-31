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

## Rare events

`tandem_overflow.modest` is the tandem queue as a CTMC with queue two emptying made absorbing, so reaching capacity is the rare overflow FIG and SENTIL also estimate. `modes` reaches it with importance splitting:

```
modest modes tandem_overflow.modest --rare FixedEffort --levels ExpectedSuccess -W 0.1 -C 0.9
```

It recovers about 5.3e-6 at c=8 against the exact 5.602e-6. The numbers are in `results/modest_rare_event.jsonl`.