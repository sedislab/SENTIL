# Reproducing the claims

Every number in [CLAIMS.md](CLAIMS.md) comes from one of four tiers.

## The CPU tier

This tier needs only a CPU and it covers the correctness claims, the full-signal and monitoring speed, the streaming cost, and the statistical coverage. Check [docker/](../docker/) for the Docker version of the commands.

```bash
make verify
```

If you want to run the cpu tier individually, 

1. Run this for the Rust tests and benchmarks.

```bash 
cargo test -p sentil -p sentil-benchmarks
```

2. Run the tests in the C ABI.

```bash
make -C sentil-ffi test-ffi
```

3. Run and check the proof of the sliding-window deque.

```bash
cd proofs && lake build
```

4. Run and verify the speed and scalability results.

```bash
cargo run --release -p sentil-benchmarks --bin sentil_runner scalability
cargo run --release -p sentil-benchmarks --bin sentil_runner streaming
python benchmarks/runners/plot.py
```

## The GPU tier

This tier needs a GPU and it covers the GPU rare-event path and the synthesis batching. Check [docker/](../docker/) for the Docker version of the commands. Our results are from running the commands on an NVIDIA A40 GPU.

```bash
cargo test --offline --no-default-features --features synthesis-gpu -- --ignored --test-threads=1
cargo run --release -p sentil-benchmarks --features gpu --bin sentil_smc_runner -- accuracy
```

## The hardware-bound and the tool-bound tiers

This file would be updated to demonstrate how to reproduce the hardware-bound and the tool-bound tier claims.

## Tolerances

The value you get by running the command needs to fall within the tolerance stated in CLAIMS.md for our claims to hold. Please submit a report if on your device it doesn't hold.