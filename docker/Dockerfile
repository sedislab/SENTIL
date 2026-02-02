# syntax=docker/dockerfile:1

FROM ubuntu:22.04 AS base

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential curl ca-certificates pkg-config git make python3 \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /sentil
COPY . .

RUN cargo build --release -p sentil -p sentil-ffi \
    && cargo test --no-run -p sentil

# The CPU claim tier: the engine suite, including the deterministic oracle and the
# no-panic fuzz, then the C ABI tests against the built libsentil, all offline.
CMD ["bash", "-lc", "cargo test --offline -p sentil && make -C sentil-ffi test-ffi CARGO='cargo --offline'"]

FROM nvidia/cuda:12.4.1-runtime-ubuntu22.04 AS gpu

ENV DEBIAN_FRONTEND=noninteractive
ENV NVIDIA_DRIVER_CAPABILITIES=compute,graphics,utility
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential curl ca-certificates pkg-config git make python3 \
      libvulkan1 vulkan-tools mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /sentil
COPY . .

RUN cargo build --release -p sentil --features gpu \
    && cargo test --no-run -p sentil --no-default-features --features synthesis-gpu \
    && cargo test -p sentil --no-default-features --features synthesis-gpu --lib -- --ignored --list \
       | grep -qx 'gpu::tests::reports_a_present_device: test'

CMD ["bash", "-lc", "if cargo test --offline -p sentil --no-default-features --features synthesis-gpu --lib -- --ignored --exact gpu::tests::reports_a_present_device >/dev/null 2>&1; then exec cargo test --offline -p sentil --no-default-features --features synthesis-gpu -- --ignored --test-threads=1; else echo 'SKIP: no GPU adapter is visible in this container, so the GPU tier did not run. Rerun on a host with an NVIDIA driver and the nvidia-container-toolkit, passing --gpus all.'; fi"]