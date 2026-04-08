# Modest SMC baseline models

We test four models on the Modest Toolset.

## The models

| File | What it models |
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

To run all four models, use `make bench-modest`. The command writes `benchmarks/results/modest_smc.jsonl`.

## Rare events

`tandem_overflow.modest` is the tandem queue as a CTMC with queue two emptying. The `modes` reaches it with importance splitting

```
modest modes tandem_overflow.modest --rare FixedEffort --levels ExpectedSuccess -W 0.1 -C 0.9
```

It recovers about 5.3e-6 at c=8 against the exact 5.602e-6. The results are in `results/modest_rare_event.jsonl`.