# SENTIL for Java

Java bindings for SENTIL, a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. The package wraps the same compiled core the C, C++, Python, and Julia bindings use, so a JVM program gets the full engine: deterministic STL monitoring, probabilistic statistical monitoring, and synthesis, with no Rust toolchain required at run time. The jar carries the native library for your platform and loads it on first use.

## What it gives you

Parse or compose a formula, evaluate its robustness over a trace offline or one sample at a time online, estimate how likely a probabilistic specification holds with rigorous confidence bounds, and synthesize an input or a controller that satisfies a specification. The specifications library ships vetted, standards-derived formulas you can reach for directly.

```java
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.Trace;

try (Formula phi = Formula.parse("always[0, 10](speed < 120)");
        Trace trace = Trace.create(new double[] {0, 1, 2, 3, 4, 5})) {
    trace.addSignal("speed", new double[] {100, 110, 125, 118, 90, 80});
    System.out.println(phi.robustness(trace));   // the margin to the boundary, negative when violated
    System.out.println(phi.violations(trace));   // the time spans where it fails
}
```

A `Formula`, `Trace`, `Monitor`, and the other native-backed types hold a handle, so use them in try-with-resources or call `close()` when done. Value types like `Robustness` and `Interval` are plain immutable objects.

## Installing

From Maven Central:

```xml
<dependency>
  <groupId>io.github.sedislab</groupId>
  <artifactId>sentil</artifactId>
  <version>0.3.0</version>
</dependency>
```

or with Gradle:

```groovy
implementation 'io.github.sedislab:sentil:0.3.0'
```

The published jar bundles the native library for Linux, macOS, and Windows on common architectures, so nothing else is needed.

## Downloading a release

If you would rather not use Maven, download the `sentil-0.3.0.jar` attached to the GitHub release and put it on the classpath directly. The jar carries the native library inside under `native/<os>-<arch>/`, so nothing else is needed. The classpath separator differs by platform: a colon on macOS and Linux, a semicolon on Windows.

On macOS or Linux:

```sh
javac -cp sentil-0.3.0.jar MyMonitor.java
java -cp .:sentil-0.3.0.jar MyMonitor
```

On Windows:

```bat
javac -cp sentil-0.3.0.jar MyMonitor.java
java -cp .;sentil-0.3.0.jar MyMonitor
```

## Building from source

Building the jar compiles the Rust core and a small JNI shim, then packages both into the jar. You need a JDK (11 or newer) whose `include` directory carries `jni.h`, CMake, a C++ compiler, and a Rust toolchain. Maven drives the whole sequence:

```sh
mvn -DskipTests package
```

That runs `cargo build --release` for the core, configures and builds the shim with CMake, and writes `target/sentil-0.3.0.jar` with the native library inside under `native/<os>-<arch>/`. The same command works on each platform: it produces `libsentil.so` on Linux, `libsentil.dylib` on macOS, and `sentil.dll` on Windows, and the loader picks the right one at run time. To assemble a single jar that runs on every platform, build on each and merge the per-platform `native/` trees, which is what the release workflow does.

## Examples

The `examples/` directory has one program per capability, the same set the other bindings ship: `OfflineMonitoring` for offline robustness in discrete and dense time, `OnlineStreaming` for the streaming monitor, `Probabilistic` for statistical monitoring, and `Synthesis` for going from a specification to a control input. Each file's header shows how to compile and run it against the jar.

## Errors

Every fallible call throws a checked `SentilException` rather than crashing: `ParseException` for a malformed formula, with the column it broke at; `SemanticException` for an input the engine cannot make sense of, such as an unknown variable or a non-probabilistic formula handed to the statistical layer; and `EvaluationException` for a runtime fault. Catch `SentilException` to handle any of them. Using a closed handle throws rather than reaching freed memory.

## Documentation

The full guide, with the API reference and worked lessons, is on the documentation site. The other bindings expose the same operations under the same names, so an example in any of them reads across.

## License

Dual licensed under MIT or Apache 2.0, at your option. See the repository root for the full texts.

## Authors

Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS lab at Baylor University.