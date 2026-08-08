<div align="center">

# SENTIL

#### The C ABI for Probabilistic Signal Temporal Logic

[![Release](https://img.shields.io/github/v/release/sedislab/SENTIL?label=release)](https://github.com/sedislab/SENTIL/releases)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

The C interface to the [`sentil`](../sentil-core) engine. You get the same deterministic monitoring, statistical model checking, and synthesis, through a C ABI. Every other language binding is built on this header, and so can yours.

You get `libsentil.{so,dylib,dll}` plus a static `libsentil.a`, and a header, `include/sentil.h`. The linker flag is `-lsentil`. Every function clears the calling thread's last error on entry; a failed call returns a sentinel (a null handle, a NaN, or a nonzero `sentil_error_t`) and leaves a code and message that `sentil_get_last_error_code` and `sentil_get_last_error` can read back.

## Your first monitor

```c
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double times[] = {0.0, 1.0, 2.0, 3.0, 4.0};
    double speed[] = {12.0, 9.0, 7.0, 4.0, 6.0};
    sentil_trace_t *trace = sentil_trace_create(times, 5);
    sentil_trace_add_signal(trace, "speed", speed, 5);

    sentil_monitor_t *monitor = sentil_monitor_parse("always (speed > 5)", NULL);
    if (monitor == NULL) {
        fprintf(stderr, "parse error: %s\n", sentil_get_last_error());
        return 1;
    }
    double robustness = 0.0;
    sentil_monitor_robustness(monitor, trace, &robustness);
    printf("robustness %.1f\n", robustness); /* robustness -1.0 */

    sentil_monitor_destroy(monitor);
    sentil_trace_destroy(trace);
    return 0;
}
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin.

## Online streaming

Fold one timed reading at a time with `sentil_monitor_update`, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```c
#include "sentil.h"
#include <math.h>
#include <stdio.h>

int main(void) {
    sentil_monitor_t *monitor = sentil_monitor_parse("always[0, 10] (x > -0.9)", NULL);
    if (monitor == NULL) {
        fprintf(stderr, "parse error: %s\n", sentil_get_last_error());
        return 1;
    }
    const char *names[] = {"x"};
    for (int t = 0; t < 60; ++t) {
        double x = sin(t * 0.3);
        sentil_robustness_t out;
        sentil_monitor_update(monitor, (double)t, names, &x, 1, &out);
        if (out.resolved && !out.satisfied) {
            printf("violated at t=%d, robustness=%.3f\n", t, out.value);
            sentil_monitor_destroy(monitor);
            return 0;
        }
    }
    printf("held over the whole stream\n");
    sentil_monitor_destroy(monitor);
    return 0;
}
```

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and fills a `sentil_smc_result_t` with the probability, a Wilson confidence interval, and the verdict.

```c
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double times[20], xs[20];
    for (int i = 0; i < 20; ++i) { times[i] = i; xs[i] = 0.4 + 0.05 * i; }
    sentil_trace_t *trace = sentil_trace_create(times, 20);
    sentil_trace_add_signal(trace, "x", xs, 20);

    /* register consumes the noise model, so it is not freed here */
    sentil_lifting_registry_t *lifting = sentil_lifting_registry_create();
    sentil_lifting_registry_register(lifting, "x", sentil_noise_gaussian(0.0, 0.3),
                                     SENTIL_NOISE_ADDITIVE);

    sentil_formula_t *phi = sentil_formula_parse("P>=0.9 (always (x > 0))");
    sentil_smc_config_t config = sentil_smc_config_default();
    config.samples = 5000;

    sentil_smc_result_t result;
    sentil_formula_check(phi, trace, lifting, &config, &result);
    printf("probability %.3f, interval [%.3f, %.3f], holds %s\n", result.probability,
           result.interval.lower, result.interval.upper, result.holds ? "true" : "false");

    sentil_formula_destroy(phi);
    sentil_lifting_registry_destroy(lifting);
    sentil_trace_destroy(trace);
    return 0;
}
```

## Specifications

The premade library is available on the C side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Build a formula, or a monitor, straight from one.

```c
sentil_spec_builder_t *spec = sentil_spec_builder_create("automotive/safe_following_distance");
spec = sentil_spec_builder_with_param(spec, "rho", 1.0); /* the follower's reaction time */
sentil_formula_t *phi = sentil_spec_builder_build_formula(spec);
/* phi monitors gap, v_r, and v_f against the RSS safe-distance bound */
sentil_spec_builder_destroy(spec);
```

List them at runtime with `sentil_spec_registry_available`, or browse them under [`specifications/`](../specifications).

## Benchmarks

We run benchmarks to compare SENTIL in C against other libraries and systems and to see the overhead the ffi imposes on the core SENTIL.

![Online streaming cost per sample: SENTIL (C ABI) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_c.png)

Per-sample streaming cost across the bindings, C and the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (C ABI) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_c.png)

Offline cost over the trace length, the C ABI and the core against RTAMT, MoonLight, and Banquo. C tracks the core, and both stay an order of magnitude below the baselines.

![Memory: the SENTIL engine streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### Package managers

vcpkg and Conan carry the C ABI on Linux, macOS, and Windows:

```bash
vcpkg install sentil
```

```bash
conan install --requires=sentil/0.3.0
```

On Linux, the release also ships distro packages you install directly; each drops `libsentil`, `sentil.h`, and the pkg-config and CMake files into the system prefix:

```bash
sudo apt install ./libsentil-dev_0.3.0-1_amd64.deb      # Debian, Ubuntu
sudo dnf install ./libsentil-devel-0.3.0-1.x86_64.rpm   # Fedora, RHEL
```

### Prebuilt release

To skip the package managers, grab the self-contained tarball for your platform from the [GitHub release](https://github.com/sedislab/SENTIL/releases). Each archive is a prefix: `lib/` holds the shared library and the static `libsentil.a`, `include/` holds `sentil.h`, and the pkg-config and CMake discovery files sit under `lib/`.

#### Linux

```bash
tar -xzf sentil-0.3.0-linux-x86_64.tar.gz
export PKG_CONFIG_PATH=$PWD/sentil-0.3.0-linux-x86_64/lib/pkgconfig:$PKG_CONFIG_PATH
export CMAKE_PREFIX_PATH=$PWD/sentil-0.3.0-linux-x86_64:$CMAKE_PREFIX_PATH
```

#### macOS

The shared library is `libsentil.dylib`; the steps match Linux with the `macos-x86_64` or `macos-arm64` archive.

```bash
tar -xzf sentil-0.3.0-macos-arm64.tar.gz
export PKG_CONFIG_PATH=$PWD/sentil-0.3.0-macos-arm64/lib/pkgconfig:$PKG_CONFIG_PATH
export CMAKE_PREFIX_PATH=$PWD/sentil-0.3.0-macos-arm64:$CMAKE_PREFIX_PATH
```

#### Windows

`tar` ships in Windows 10 and later. There is no pkg-config, so set `CMAKE_PREFIX_PATH` and let CMake find the package; MSVC links `sentil.dll` through the bundled import library.

```bash
tar -xzf sentil-0.3.0-windows-x86_64.tar.gz
$env:CMAKE_PREFIX_PATH = "$PWD\sentil-0.3.0-windows-x86_64"
```

### Build from source

You need a Rust toolchain (pinned in `rust-toolchain.toml`) and a C compiler.

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL/sentil-ffi
make            # build the release cdylib and staticlib
make test-ffi   # compile, link, and run the C tests against it
make leakcheck  # the same under valgrind
```

`make build` runs `cargo build --release -p sentil-ffi` and leaves the artifacts in `target/release`.

### Linking

The library ships pkg-config and CMake discovery files. After a package-manager install, or `make install PREFIX=/your/prefix`:

```bash
cc app.c $(pkg-config --cflags --libs sentil) -o app
```

```cmake
find_package(Sentil REQUIRED)
target_link_libraries(app PRIVATE Sentil::sentil)
```

Without installing, point the compiler at the build tree: `-Ipath/to/sentil-ffi/include -Lpath/to/target/release -lsentil`.

## Memory

Every `_create` and builder pairs with one `_destroy`. Strings come back owned and are freed with `sentil_free_string`; arrays with the typed free the header names for each (`sentil_free_doubles`, `sentil_free_string_array`, `sentil_free_samples`, `sentil_free_intervals`, `sentil_free_robustness`, `sentil_free_named_robustness`, `sentil_free_bank_results`). The builders that take handles consume them, even on a failed return, so a caller never double-frees an operand.

## Contributing

The C ABI builds with the Cargo workspace, so no external core is needed. Run the Rust and C tests for a change:

```
cargo test -p sentil-ffi
make -C sentil-ffi test-ffi
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The header is the reference; the [documentation site](https://sentil.pages.dev) carries the C and C++ guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/start/tutorial). The C++ wrapper `sentil.hpp` over this header lives in [`sentil-cpp`](../sentil-cpp).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Temporal Logic},
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