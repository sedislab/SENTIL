# PRISM SMC baseline model

The circadian network as a model for PRISM. SENTIL, UPPAAL-SMC and Modest check the same network with the same property, so you can compare their numbers directly.

## The model

`circadian.nm` holds the Barkai-Leibler gene network as a CTMC, with the seven reactions and their rates. The property is in the runner: the activator reaches 100 within 20 time units.

## Running it

Get PRISM from `prismmodelchecker.org`. Then install it.

Set `PRISM` to the launcher. Then run the model:

```
PRISM=<path>/prism bash benchmarks/runners/prism_runner.sh benchmarks/baselines/prism/circadian.nm
```

You can also use `make bench-prism` to run everything. It writes `benchmarks/results/prism_smc.jsonl`.
