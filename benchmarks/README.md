# SENTIL benchmarks

We test the performance of SENTIL against a variety of tools on several benchmarks.

## Directory Structure

The signals, formulas and their robustness results are in `deterministic/`. The stochastic models and their probability values are in `src/probabilistic.rs`. `sentil_runner` generates the length, depth, and bound sweeps. The code that implement each baseline tool are in `runners/` and the result json files and plots are in `results/`.

## Running

Run a track with `cargo run --release -p sentil-benchmarks --bin sentil_runner <evaluation_area>`, where `<evaluation_area>` can be `scalability`, `deterministic`, `dense`, or `streaming`, then draw the figures with `python runners/plot.py`. The `sentil_synth_runner` and `sentil_particle_runner` bins run the synthesis and particle sweeps. A list of all experiments run and their results are described in [docs/REPRODUCE.md](../docs/REPRODUCE.md).

## Statistical model checking

`sentil_smc_runner` estimates a model's satisfaction probability. Run it on CPU with `cargo run --release --bin sentil_smc_runner accuracy` or `... throughput <samples>`. The same commands apply when you want to run it on the GPU. We run our experiments on a 128-core AMD EPYC 7763 (`sentil_smc_cpu.jsonl`) and an NVIDIA A40 (`sentil_smc_gpu.jsonl`). The 128-core CPU runs the ten-step models near 1.2 billion realization-steps per second at ten million samples and settles close to 1.9 billion by a billion. The A40 starts at 290 million realizations per second on the ten-step models at ten million samples and reaches about 2.5 billion at a hundred million and 8 billion at a billion, then plateaus near 11 billion from ten billion through a hundred billion, roughly six times the CPU at that scale, while the single-step model tops out near 2.9 billion.

## Baselines

The STL baselines all run the same formulas and reproduce its robustness exactly, so every comparison is like-for-like and only the speed and memory differ. For the STL evaluation, we benchmarked RTAMT ([`rtamt_runner.py`](runners/rtamt_runner.py)), MoonLight ([`moonlight_runner.py`](runners/moonlight_runner.py)), Banquo ([`banquo_runner.py`](runners/banquo_runner.py)) and Breach ([`breach_runner.m`](runners/breach_runner.m)). For the statistical model checking and rare-event estimation, we benchmarked FIG ([`fig/`](baselines/fig/)), Modest ([`modest/`](baselines/modest/)), UPPAAL-SMC ([`uppaal/`](baselines/uppaal/)) and PRISM ([`prism/`](baselines/prism/)). The results from running the benchmarks are in [`results/`](results/) and it's named as `<tool_name>_<evaluation_area>.jsonl`.