# sentil-cpp

C++ bindings for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. The engine is the compiled Rust core; this layer is a header-only RAII wrapper over its C ABI, so a C++ program gets the same numbers the core computes, with C++ types, exceptions, and ownership.

## What you get

One header, `sentil/sentil.hpp`. Every wrapper owns its C handle and frees it in the destructor, so you never call a destroy function by hand. Every fallible call throws a `sentil::SentilError` carrying the core's message rather than returning a status code. The names match the Python binding concept for concept, so an API you learned in one carries over.

```cpp
#include <sentil/sentil.hpp>
#include <iostream>

int main() {
    sentil::Trace trace({0, 1, 2, 3}, "speed", {12, 9, 7, 4});
    sentil::Formula phi = sentil::Formula::parse("always (speed > 5)");
    std::cout << phi.robustness(trace) << "\n";  // negative: violated by the end
}
```

Formulas also compose with operators and a small expression type:

```cpp
using sentil::Expr;
auto phi = sentil::always(Expr::var("speed") > 5) && sentil::eventually(Expr::var("gap") > 2);
```

## Install

The C library and its header come from the SENTIL C package. Once that is installed, `sentil-cpp` adds the header on top, found two ways so a project can pick either.

With CMake:

```cmake
find_package(SentilCpp CONFIG REQUIRED)
target_link_libraries(my_app PRIVATE Sentil::cpp)
```

With pkg-config the C library resolves as `sentil`, and the C++ headers sit beside it:

```
c++ my_app.cpp $(pkg-config --cflags --libs sentil) -o my_app
```

The packages ship through vcpkg and Conan as `sentil`, and the C library ships as `.deb` and `.rpm` (`libsentil-dev`) for apt, yum, and pacman. On macOS and Windows the same CMake and vcpkg paths work; the shared library is `libsentil.dylib` and `sentil.dll`.

## Prebuilt release from GitHub

If you would rather not go through a package manager, download the release archive from the SENTIL releases page. The same `sentil-0.3.0-<os>-<arch>.tar.gz` bundle that carries the C ABI also carries the C++ header, so one archive covers both. Extract it and point `CMAKE_PREFIX_PATH` at the extracted prefix; `find_package(Sentil)` then resolves the C library and the bundled `sentil.hpp` beside it.

On Linux:

```
tar xzf sentil-0.3.0-linux-x86_64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD/sentil-0.3.0-linux-x86_64"
```

On macOS:

```
tar xzf sentil-0.3.0-macos-arm64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD/sentil-0.3.0-macos-arm64"
```

On Windows, extract with the bundled `tar` and pass the prefix from PowerShell:

```
tar xzf sentil-0.3.0-windows-x86_64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD\sentil-0.3.0-windows-x86_64"
```

## Build from source

You need CMake 3.16 or newer, a C++17 compiler, and the Rust toolchain to build the core. From the repository root:

```
cmake -S sentil-cpp -B sentil-cpp/build
cmake --build sentil-cpp/build
ctest --test-dir sentil-cpp/build
```

The build compiles `libsentil` first, so a test never runs against a stale core. `cmake --build sentil-cpp/build --target leakcheck` runs the suite under valgrind and expects zero definite or indirect leaks. The four programs under `examples/` build alongside the tests and run unmodified.

## What it covers

The whole engine, with nothing the Rust core can do left out: deterministic STL monitoring offline and online, the statistical layer (noise models, lifting, statistical model checking, the sequential tests, rare-event splitting), synthesis (the smooth robustness and its gradients, the numerics, open-loop synthesis, the receding-horizon controller, the safety filter, chance constraints, and the counterexample search), and the specifications library of vetted PrSTL specs you reach by name with `SpecBuilder` (`SpecBuilder::available()` lists them). Beyond the declarative paths it also wraps the hooks that take a host callback: optimizing your own objective with maximize and CMA-ES, a stochastic system or a system model whose dynamics are your own function, the adaptive-multilevel-splitting interface over your own simulator, the sequential tests over your own Bernoulli source, and parameter mining. The engine runs those callbacks across worker threads, so a callback must be thread-safe; an exception it throws is caught and resurfaced as a normal C++ error rather than unwinding through the engine.

## More

Per-language guides and the full reference live at the documentation site. SENTIL is by Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University, dual licensed under MIT or Apache-2.0.