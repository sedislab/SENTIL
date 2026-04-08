# UPPAAL-SMC baseline model

The circadian network as a model for UPPAAL-SMC.

## The model

The Barkai-Leibler gene network is in `circadian.xml` and the property (the activator reaches 100 within 20 time units) is in `circadian.q`.

## Running it

Get UPPAAL from `uppaal.org`. Then install it.

Set `VERIFYTA` to the binary. Then run the model:

```
VERIFYTA=<path>/verifyta bash benchmarks/runners/uppaal_runner.sh benchmarks/baselines/uppaal/circadian.xml
```

You can also use `make bench-uppaal` to run it and it'll write `benchmarks/results/uppaal_smc.jsonl`.

UPPAAL selects its own number of runs and stops when the interval is as narrow as the specification requires.