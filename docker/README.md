# Docker

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

From the repository root, build the base image and run the CPU claim tier:

```bash
docker compose -f docker/docker-compose.yml run --rm sentil-verify
```

This builds the core, runs the engine suite including the deterministic oracle and the no-panic fuzz, and runs the C ABI tests against the built library, all offline. A clean exit means the CPU tier reproduces.

## GPU reproduction

The GPU rare-event and synthesis-batching paths need an NVIDIA GPU reached through the [nvidia-container-toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html). On a host with a GPU and the toolkit installed:

```bash
docker compose -f docker/docker-compose.yml run --rm sentil-gpu
```

or, with plain docker:

```bash
docker build -f docker/Dockerfile --target gpu -t ghcr.io/sedislab/sentil-artifact:0.3.0-gpu .
docker run --gpus all ghcr.io/sedislab/sentil-artifact:0.3.0-gpu
```

The GPU stage is verified on a machine with a GPU, not in continuous integration, because the hosted runners have no device.

## The UPPAAL-SMC baseline

UPPAAL is licensed for academic use and may not be redistributed either, so mount a local install the same way:

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

The same verification runs directly through the Makefile and cargo: `cargo test -p sentil` for the engine suite and `make -C sentil-ffi test-ffi` for the C ABI. The Modest baseline runs the same way with `make bench-modest MODEST=<path>/modest`.