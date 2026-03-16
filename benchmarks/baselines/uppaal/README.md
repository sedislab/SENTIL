# UPPAAL-SMC baseline model

The circadian network as a model for UPPAAL-SMC. SENTIL, PRISM and Modest check the same network with the same property, so you can compare their numbers directly.

## The model

The Barkai-Leibler gene network is in `circadian.xml` and the property (the activator reaches 100 within 20 time units) is in `circadian.q`.

## Running it

Get UPPAAL from `uppaal.org`. Then install it.

Set `VERIFYTA` to the binary. Then run the model:

```
VERIFYTA=<path>/verifyta bash benchmarks/runners/uppaal_runner.sh benchmarks/baselines/uppaal/circadian.xml
```

You can also use `make bench-uppaal` to run it and it'll write `benchmarks/results/uppaal_smc.jsonl`.

If you do not install UPPAAL, the runner prints a message and stops with success.