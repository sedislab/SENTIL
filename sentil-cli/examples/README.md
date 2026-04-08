# CLI examples

Examples to get you started and running with SENTIL's cli. Each script will generate its own sample data and run `sentil`.

- `offline_monitoring.sh`: robustness over a recorded trace, discrete and dense.
- `online_streaming.sh`: robustness over a trace that comes in per sample.
- `probabilistic.sh`: lift a noisy sensor and estimate the satisfaction probability.
- `synthesis.sh`: synthesize a control input that satisfies a spec on a linear model.

## Running

With `sentil` on your PATH (from a package install), run one directly:

```bash
./offline_monitoring.sh
```

To run against a build without installing, point `SENTIL` at the binary:

```bash
SENTIL=../../target/release/sentil ./offline_monitoring.sh
```

A script exits with the verb's exit code, so `offline_monitoring.sh` and `probabilistic.sh` exit `10` when the specification is violated or the probability falls short. Run `sentil explain exit-codes` to see all of them.