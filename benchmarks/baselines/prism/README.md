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

## Rare events

`tandem_overflow.prism` and `tandem_overflow.props` are the tandem queue FIG is built around, with queue two emptying made absorbing so that reaching capacity is the rare overflow. PRISM computes it exactly, `P=? [ q2>0 U q2=c ]`, giving 5.602e-6 at c=8 in a few milliseconds. That is the reference value the simulation tools, SENTIL, FIG and Modest, are scored against; exact model checking is the right tool here because the state space is small, and it stops being an option once the model is large or continuous.

```
prism tandem_overflow.prism tandem_overflow.props
```