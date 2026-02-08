# CLI examples

The canonical set as small shell scripts, the same programs the other bindings ship. Each generates its own sample data, runs `sentil`, and cleans up after itself.

- `offline_monitoring.sh`: robustness over a recorded trace, discrete and dense.
- `online_streaming.sh`: feed one JSON sample per line into the monitor.
- `probabilistic.sh`: lift a noisy sensor and estimate the satisfaction probability.
- `synthesis.sh`: synthesize a control input that satisfies a spec on a linear model.

## Running

With `sentil` on your PATH (from a package install), run one directly:

```
./offline_monitoring.sh
```

To run against a build without installing, point `SENTIL` at the binary:

```
SENTIL=../../target/release/sentil ./offline_monitoring.sh
```

A script exits with the verb's exit code, so `offline_monitoring.sh` and `probabilistic.sh` exit `10` when the specification is violated or the probability falls short. That is a verdict, not an error; `sentil explain exit-codes` lists them.