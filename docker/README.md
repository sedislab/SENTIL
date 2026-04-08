# Reproducing SENTIL's paper results with Docker

We've prepared a Docker image that allows you to reproduce SENTIL's claims. The base stage reproduces and verifies the hardware-independent tests on a CPU and the gpu stage reproduces the empirical tests and benchmarks on an NVIDIA GPU.

## Pull the image

```bash
docker run --rm ghcr.io/sedislab/sentil-artifact:0.3.0
```

To run the gpu stage, you need to append the command with the `-gpu` suffix. Use `latest` and `latest-gpu` track the most recent release:

```bash
docker run --rm --gpus all ghcr.io/sedislab/sentil-artifact:0.3.0-gpu
```

You can also compose the image yourself to run specific tests or benchmarks. Instructions for that are below.

## CPU verification

From the root of the repo, run

```bash
docker compose -f docker/docker-compose.yml run --rm sentil-verify
```

This builds SENTIL, runs all the tests and the deterministic benchmarks.

## GPU reproduction

The GPU rare-event and synthesis-batching paths need an NVIDIA GPU reached through the [nvidia-container-toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html). On a host with a GPU and the toolkit installed,

```bash
docker compose -f docker/docker-compose.yml run --rm sentil-gpu
```

or, with plain docker:

```bash
docker build -f docker/Dockerfile --target gpu -t ghcr.io/sedislab/sentil-artifact:0.3.0-gpu .
docker run --gpus all ghcr.io/sedislab/sentil-artifact:0.3.0-gpu
```

## Statistical Model Checking verification

To run the SMC benchmarks and tests, you'll need to install the various benchmark tools and then run the benchmarks.

### PRISM

1. Download the Linux release from [prismmodelchecker.org](prismmodelchecker.org).
2. Unpack it, get into the directory and then run ./install.sh.
3. The launcher is placed at <prism-dir>/bin/prism. Check it with <prism-dir>/bin/prism -version.
4. PRISM_HOME=<prism-dir>/bin/prism.
5. Run `PRISM_HOME=/path/to/prism docker compose -f docker/docker-compose.yml run --rm sentil-prism`.

### UPPAAL-SMC

1. Request a license and download the Linux build from [uppaal.org](uppaal.org).
2. Unpack it. The binary is <uppaal-dir>/bin-Linux/verifyta.
3. Make it executable if needed, and check with <uppaal-dir>/bin-Linux/verifyta -v.
4. UPPAAL_HOME=<uppaal-dir>/bin-Linux/verifyta.
5. Run `UPPAAL_HOME=/path/to/uppaal docker compose -f docker/docker-compose.yml run --rm sentil-uppaal`.

### Modest

1. Download the Linux build from [modestchecker.net](modestchecker.net).
2. Unpack it and run <modest-dir>/modest.
3. Check with <modest-dir>/modest --version.
4. MODEST_HOME=<modest-dir>/modest.
5. Run `MODEST_HOME=/path/to/Modest docker compose -f docker/docker-compose.yml run --rm sentil-modest`.

## Without Docker

If you don't have Docker (mostly for the academics on HPCs), you can do the reproduction through Makefile. Run `make verify` for CPU verification. And for the PRISM, UPPAAL-SMC and Modest SMC benchmarks, run `make bench-prism PRISM=<prism-dir>/bin/prism`, `make bench-uppaal VERIFYTA=<uppaal-dir>/bin-Linux/verifyta` and `make bench-modest MODEST=<modest-dir>/modest` respectively. Check the [Makefile](../Makefile) for detailed instructions.