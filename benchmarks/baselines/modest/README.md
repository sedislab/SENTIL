# Modest SMC baseline models

We test four models on the Modest Toolset. Each model gives a statistical model checking baseline for the SENTIL statistical layer. PRISM and SENTIL check the circadian model network with the same property, so the three tools give comparable numbers.

## The models

| File | What it models | Ported from | Change made |
| --- | --- | --- | --- |
| `circadian.modest` | Barkai-Leibler gene network, a CTMC | the PRISM model | reactions become rate transitions |
| `tandem_queue.modest` | two queues in series, a CTMC | the PRISM model | reactions become rate transitions |
| `biodiesel.modest` | a reactor with a heater that can fail | the reference model | functions replace the local variables |
| `powertrain.modest` | air-fuel-ratio control | the reference model | one step becomes three blocks |

## Running them

Install the Modest Toolset first. Get the toolset from `modestchecker.net`.

Set `MODEST` to the launcher. Then run one model:

```
MODEST=<path>/modest bash benchmarks/runners/modest_runner.sh benchmarks/baselines/modest/circadian.modest
```

The runner does 10,000 simulation runs with a fixed seed. It writes one JSON record to standard output.

To run all four models, use `make bench-modest`. The command writes `benchmarks/results/modest_smc.jsonl`.