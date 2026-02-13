# Reproducing the claims

Every number in [CLAIMS.md](CLAIMS.md) comes from one of three tiers. The first runs anywhere in minutes, the second needs a GPU, and the third is bound to specific hardware and is recorded rather than rerun on a different machine. Reproduce what your hardware supports; the tier is stated for each claim.

## The CPU tier: correctness and the hardware-independent speed numbers

This tier needs only a CPU and a Rust toolchain, and finishes in minutes. It covers the correctness claims, the full-signal and monitoring speed, the streaming cost, and the statistical coverage.

The engine suite, including the deterministic oracle, the deque-equals-naive equivalence, the confidence-interval coverage, and the no-panic fuzz:

```bash 
cargo test -p sentil -p sentil-benchmarks
```

The C ABI, built and linked against every C test:

```
make -C sentil-ffi test-ffi
```

The Lean proof of the sliding-window deque, machine-checked and axiom-clean:

```
cd proofs && lake build
```

The speed and scalability numbers, from the benchmark runner, which takes a suite name and writes JSON under `benchmarks/results/` that the plotter reads:

```
cargo run --release -p sentil-benchmarks --bin sentil_runner scalability
cargo run --release -p sentil-benchmarks --bin sentil_runner streaming
python benchmarks/runners/plot.py
```

In Docker, the whole CPU tier runs offline in one command from the repository root:

This tier needs a GPU and it covers the GPU rare-event path and the synthesis batching. Check [docker/](../docker/) for the Docker version of the commands. Our results are from running the commands on an NVIDIA A40 GPU.

```bash
cargo test --offline --no-default-features --features synthesis-gpu -- --ignored --test-threads=1
cargo run --release -p sentil-benchmarks --features gpu --bin sentil_smc_runner -- accuracy
```

## The hardware-bound and the tool-bound tiers

Some numbers are tied to one machine and cannot be reproduced faithfully elsewhere: the Raspberry Pi 4 embedded latency and the A100 rare-event timings. CLAIMS.md records these with the hardware they were measured on and the conditions under which they hold, following the measurements taken on that hardware. Rerunning them needs the same device.

## Tolerances

A claim is confirmed when its regenerated value falls within the tolerance stated in CLAIMS.md. A value outside tolerance is a regression to investigate, not a number to overwrite.