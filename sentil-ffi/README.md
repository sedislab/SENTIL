# sentil-ffi

The stable C ABI for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. It exposes the whole `sentil` engine, deterministic monitoring, statistical model checking, and synthesis, through a flat C interface: opaque handles and an errno-style last error, so a fault returns a code rather than aborting the host process.

## What you get

`libsentil.{so,dylib,dll}` plus a static `libsentil.a`, and one hand-written header, `include/sentil.h`. The linker flag is `-lsentil`. Every function clears the calling thread's last error on entry; a failed call returns a sentinel (a null handle, a NaN, or a nonzero `sentil_error_t`) and leaves a code and message that `sentil_get_last_error_code` and `sentil_get_last_error` read back.

## Build from source

You need a Rust toolchain (the version is pinned in `rust-toolchain.toml`) and a C compiler.

```
make            # build the release cdylib and staticlib
make test-ffi   # compile, link, and run the C tests against it
make leakcheck  # the same under valgrind
```

`make build` runs `cargo build --release -p sentil-ffi` and leaves the artifacts in `target/release`.

## Linking

The library ships pkg-config and CMake discovery files. After `make install PREFIX=/your/prefix`:

pkg-config:

```
cc app.c $(pkg-config --cflags --libs sentil) -o app
```

CMake:

```cmake
find_package(Sentil REQUIRED)
target_link_libraries(app PRIVATE Sentil::sentil)
```

Without installing, point the compiler at the build tree: `-Ipath/to/sentil-ffi/include -Lpath/to/target/release -lsentil`.

### Platforms

The Linux path above is the tested one. On macOS the shared library is `libsentil.dylib` and the same `make` and pkg-config steps apply. On Windows, build with `cargo build --release -p sentil-ffi` and link `sentil.dll` through its import library from MSVC; the header is unchanged.

## A first monitor

```c
#include "sentil.h"
#include <stdio.h>

int main(void) {
    double times[] = {0.0, 1.0, 2.0, 3.0};
    double speed[] = {12.0, 9.0, 7.0, 4.0};
    sentil_trace_t *trace = sentil_trace_create(times, 4);
    sentil_trace_add_signal(trace, "speed", speed, 4);

    sentil_monitor_t *monitor = sentil_monitor_parse("always (speed > 5)", NULL);
    double robustness = 0.0;
    if (sentil_monitor_robustness(monitor, trace, &robustness) == SENTIL_OK) {
        printf("robustness %.3f, %s\n", robustness, robustness >= 0 ? "holds" : "fails");
    } else {
        printf("error: %s\n", sentil_get_last_error());
    }
    sentil_monitor_destroy(monitor);
    sentil_trace_destroy(trace);
    return 0;
}
```

## Memory

Every `_create` and builder pairs with one `_destroy`. Strings come back owned and are freed with `sentil_free_string`; arrays with the typed free the header names for each (`sentil_free_doubles`, `sentil_free_string_array`, `sentil_free_samples`, `sentil_free_intervals`, `sentil_free_robustness`, `sentil_free_named_robustness`, `sentil_free_bank_results`). The builders that take handles consume them, even on a failed return, so a caller never double-frees an operand.

## More

Per-language guides and runnable examples live at the documentation site. SENTIL is by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University, dual licensed under MIT or Apache-2.0.