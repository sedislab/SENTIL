# C examples

The canonical set, the same programs the other bindings ship, written against the C ABI. Each is one small `main` and prints its result.

- `offline_monitoring.c`: robustness over a recorded trace, discrete and dense.
- `online_streaming.c`: fold one timestamped sample at a time.
- `probabilistic.c`: lift a noisy sensor and estimate the satisfaction probability.
- `synthesis.c`: synthesize a control input that satisfies a spec on a linear model.

## Build and run

We use `make` for a quick build and running of every example:

```bash
make examples
```

To build one by hand, first build the library (`make build`), then compile against it. The library lives in `../target/release` from `sentil-ffi/`; point the linker and the runtime loader at it.

Linux:

```bash
cc -Iinclude examples/offline_monitoring.c -L../target/release -lsentil -Wl,-rpath,../target/release -lm -o offline
./offline
```

macOS is the same with `-Wl,-rpath,../target/release` resolving `libsentil.dylib`. On Windows, build with the MSVC toolchain, link against `sentil.dll.lib`, and put `sentil.dll` on the `PATH` or beside the executable:

```bash
cl /I include examples\offline_monitoring.c /link /LIBPATH:..\target\release sentil.dll.lib
```

Once the library is installed under a prefix (`make install`, or from a package), the header and library are found the usual way: `cc $(pkg-config --cflags --libs sentil) examples/offline_monitoring.c -o offline`.