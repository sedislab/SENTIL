<div align="center">

# SENTIL

#### The C++ library for Probabilistic Signal Temporal Logic

[![Release](https://img.shields.io/github/v/release/sedislab/SENTIL?label=release)](https://github.com/sedislab/SENTIL/releases)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

C++ bindings for the [`sentil`](../sentil-core) engine. It provides a header, `sentil/sentil.hpp` that allows you to do deterministic monitoring, statistical model checking, and synthesis.

## Your first monitor

```cpp
#include <sentil/sentil.hpp>
#include <iostream>

int main() {
    sentil::Trace trace({0, 1, 2, 3, 4}, "speed", {12.0, 9.0, 7.0, 4.0, 6.0});
    sentil::Formula phi = sentil::Formula::parse("always (speed > 5)");
    std::cout << "robustness " << phi.robustness(trace) << "\n";  // robustness -1
}
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin. Formulas also compose with operators and a small expression type:

```cpp
using sentil::Expr;
auto phi = sentil::always(Expr::var("speed") > 5) && sentil::eventually(Expr::var("gap") > 2);
```

## Online streaming

An `OnlineMonitor` folds one timed reading at a time, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```cpp
#include <sentil/sentil.hpp>
#include <cmath>
#include <cstdio>

int main() {
    sentil::OnlineMonitor monitor("always[0, 10] (x > -0.9)");
    for (int t = 0; t < 60; ++t) {
        double x = std::sin(t * 0.3);
        sentil::Robustness verdict = monitor.update(t, {{"x", x}});
        if (verdict.resolved && !verdict.satisfied) {
            std::printf("violated at t=%d, robustness=%.3f\n", t, verdict.value);
            return 0;
        }
    }
    std::printf("held over the whole stream\n");
}
```

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and returns the probability with a Wilson confidence interval.

```cpp
#include <sentil/sentil.hpp>
#include <cstdio>
#include <vector>

int main() {
    std::vector<double> times, values;
    for (int i = 0; i < 20; ++i) { times.push_back(i); values.push_back(0.4 + 0.05 * i); }
    sentil::Trace trace(times, "x", values);

    sentil::LiftingRegistry lifting;
    lifting.register_noise("x", sentil::NoiseModel::gaussian(0.0, 0.3));

    sentil::Formula phi = sentil::Formula::parse("P>=0.9 (always (x > 0))");
    sentil::SmcConfig config;
    config.samples = 5000;

    sentil::SmcResult result = phi.check(trace, lifting, config);
    std::printf("probability %.3f, interval [%.3f, %.3f], holds %s\n", result.probability,
                result.interval.lower, result.interval.upper, result.holds ? "true" : "false");
}
```

## Specifications

The premade library is on the C++ side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Build a formula, or a monitor, straight from one.

```cpp
auto phi = sentil::SpecBuilder("automotive/safe_following_distance")
    .with_param("rho", 1.0)  // the follower's reaction time
    .build_formula();
// phi monitors gap, v_r, and v_f against the RSS safe-distance bound
```

List them with `sentil::SpecBuilder::available()`, or browse them under [`specifications/`](../specifications).

## Benchmarks

The wrapper is header-only over the C ABI, so it runs at the core's speed. These plots put C++ and the Rust core against the baseline tools, from the same runs.

![Online streaming cost per sample: SENTIL (C++) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_cpp.png)

Per-sample streaming cost across the bindings, C++ and the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (C++) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_cpp.png)

Offline cost over the trace length, C++ and the core against RTAMT, MoonLight, and Banquo, an order of magnitude below them.

![Memory: SENTIL (C++) streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory_cpp.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

The C++ library needs two pieces: `libsentil` and its header (from the SENTIL C package), and this `sentil/sentil.hpp` on top.

### Package managers

vcpkg and Conan carry the C++ wrapper on Linux, macOS, and Windows:

```bash
vcpkg install sentil-cpp
```

```bash
conan install --requires=sentil-cpp/0.3.0
```

On Linux, the C library also ships as a distro package; the header then comes from vcpkg, Conan, or a checkout:

```bash
sudo apt install ./libsentil-dev_0.3.0_amd64.deb      # Debian, Ubuntu
sudo dnf install ./libsentil-devel-0.3.0.x86_64.rpm   # Fedora, RHEL
```

### Prebuilt release

The `sentil-0.3.0-<os>-<arch>.tar.gz` bundle on the [GitHub release](https://github.com/sedislab/SENTIL/releases) carries the C ABI (the libraries, `sentil.h`, the pkg-config and CMake files). The C++ header is not in it; bring `sentil/sentil.hpp` from vcpkg, Conan, or a checkout of `sentil-cpp`. Extract the bundle and point `CMAKE_PREFIX_PATH` at it so the C library resolves.

#### Linux

```bash
tar -xzf sentil-0.3.0-linux-x86_64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD/sentil-0.3.0-linux-x86_64"
```

#### macOS

```bash
tar -xzf sentil-0.3.0-macos-arm64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD/sentil-0.3.0-macos-arm64"
```

#### Windows

`tar` ships in Windows 10 and later; pass the prefix from PowerShell.

```bash
tar -xzf sentil-0.3.0-windows-x86_64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD\sentil-0.3.0-windows-x86_64"
```

### Build from source

You need CMake 3.16 or newer, a C++17 compiler, and the Rust toolchain to build the core.

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL
cmake -S sentil-cpp -B sentil-cpp/build
cmake --build sentil-cpp/build
ctest --test-dir sentil-cpp/build
```

The build compiles `libsentil` first, so a test never links a stale core. `cmake --build sentil-cpp/build --target leakcheck` runs the suite under valgrind, and the four programs under `examples/` build alongside the tests.

### Linking

With CMake:

```cmake
find_package(SentilCpp CONFIG REQUIRED)
target_link_libraries(my_app PRIVATE Sentil::cpp)
```

With pkg-config the C library resolves as `sentil`, and the C++ headers sit beside it:

```bash
c++ -std=c++17 my_app.cpp $(pkg-config --cflags --libs sentil) -o my_app
```

## Contributing

The build under Build from source compiles `libsentil` first, so `ctest --test-dir sentil-cpp/build` runs a change against a fresh core. The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The header is the reference; the [documentation site](https://sentil.pages.dev) carries the C and C++ guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial). The wrapper covers the whole engine: deterministic and probabilistic monitoring, synthesis, the specifications library, and the host-callback hooks (your own objective, a stochastic system whose dynamics are your function, your own simulator for rare-event splitting).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Signal Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.