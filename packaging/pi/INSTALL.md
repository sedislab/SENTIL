# Installing SENTIL on a Raspberry Pi

This archive holds a prebuilt SENTIL for your Pi's architecture. The full archive carries the `sentil` command-line tool and `libsentil` for linking C and C++ on the device. The static archive carries only the command-line tool, one file with no library dependencies, for a sensor loop.

## The command-line tool

Copy the binary onto your path and you are done:

```
sudo cp bin/sentil /usr/local/bin/
sentil --version
```

Shell completions and the man page are under `completions/` and `man/`. To install them on a typical Pi OS:

```
sudo cp completions/sentil.bash /usr/share/bash-completion/completions/sentil
sudo cp man/sentil.1 /usr/local/share/man/man1/
```

## The library, for linking on the device

Extract under `/usr/local` and refresh the linker cache:

```
sudo cp lib/libsentil.so lib/libsentil.a /usr/local/lib/
sudo cp include/sentil.h /usr/local/include/
sudo cp lib/pkgconfig/sentil.pc /usr/local/lib/pkgconfig/
sudo ldconfig
```

Then `pkg-config` reports the flags and the linker finds the library:

```
pkg-config --cflags --libs sentil
cc my_monitor.c $(pkg-config --cflags --libs sentil) -o my_monitor
```

CMake projects find it through the same prefix with `find_package(Sentil)` once the package config is on `CMAKE_PREFIX_PATH`.

## Python

The Python package carries its own compiled core, so it does not link `libsentil`. Install it from PyPI instead:

```
pip install sentil
```

Prebuilt wheels cover the 64-bit Pi OS; on the 32-bit OS pip builds the core from source and needs a Rust toolchain.

## A first monitor

```
sensor | sentil monitor -f 'always (temperature < 80)'
```

The streaming monitor reads one JSON sample per line, each with a numeric `time`, and prints a verdict per line, so `sensor` should emit lines like `{"time": 0, "temperature": 79.5}`. It holds flat per-sample cost, which is what makes it fit a real-time loop on the Pi.

## Which archive matches your board

The 64-bit Pi OS on a Pi 3, 4, 5, or Zero 2 W uses the `aarch64-unknown-linux-gnu` archive for the full set, or `aarch64-unknown-linux-musl` for the static tool. The 32-bit Pi OS uses `armv7-unknown-linux-gnueabihf`. The Pi 4 is the reference board for the published embedded latency numbers; see `docs/CLAIMS.md`.