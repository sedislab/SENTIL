# PRISM SMC baseline model

The circadian network as a model for PRISM.

## The model

`circadian.nm` holds the Barkai-Leibler gene network as a CTMC. The property is that the activator reaches 100 within 20 time units.

## Running it

Get PRISM from `prismmodelchecker.org`. Then install it.

Set `PRISM` to the launcher. Then run the model:

```
PRISM=<path>/prism bash benchmarks/runners/prism_runner.sh benchmarks/baselines/prism/circadian.nm
```

You can also use `make bench-prism` to run everything. It writes `benchmarks/results/prism_smc.jsonl`.

## Rare events

PRISM computes `P=? [ q2>0 U q2=c ]` and it gives 5.602e-6 at c=8.

```
prism tandem_overflow.prism tandem_overflow.props
```