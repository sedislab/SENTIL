<div align="center">

# SENTIL

#### The Java package for probabilistic Signal Temporal Logic

[![Maven Central](https://img.shields.io/maven-central/v/io.github.sedislab/sentil.svg)](https://central.sonatype.com/artifact/io.github.sedislab/sentil)
[![Java](https://img.shields.io/badge/Java-%E2%89%A511-blue.svg)](https://adoptium.net)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

Java bindings for the [`sentil`](../sentil-core) engine. Add the dependency and import it. The jar carries the compiled core for your platform and loads it on first use.

SENTIL has three main capabilities. Deterministic STL monitoring, offline over a recorded trace or streaming one sample at a time. Probabilistic monitoring, which fits a noise model to sensor data and estimates satisfaction probability with confidence bounds. And synthesis, from a specification to a control input to an online controller.

## Your first monitor

```java
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.Trace;

try (Trace trace = Trace.create(new double[] {0, 1, 2, 3, 4});
        Formula phi = Formula.parse("always (speed > 5)")) {
    trace.addSignal("speed", new double[] {12, 9, 7, 4, 6});
    System.out.println(phi.robustness(trace));   // -1.0
}
```

The robustness is `-1` because the speed dips to `4` at `t = 3`, one unit under the bound, so the property fails by exactly one. A non-negative value would mean it holds, and the magnitude is the margin. The per-sample signal and the violated spans are one call away:

```java
phi.robustnessSignal(trace);   // the robustness at each sample
phi.violations(trace);         // the [start, end] spans where it fails
```

A `Formula`, `Trace`, `Monitor`, and the other native-backed types hold a handle, so use them in try-with-resources or call `close()` when done; value types like `Robustness` and `Interval` are plain immutable objects. Every fallible call throws a checked `SentilException`, with `ParseException`, `SemanticException`, and `EvaluationException` for the parse, input, and runtime faults, each carrying the message from the core.

## Online streaming

An `OnlineMonitor` folds one timed reading at a time, at O(1) amortized cost per sample and memory that scales with the window, not the length of the trace. The verdict carries `resolved`, `satisfied`, and `value`, so you can watch a live system and stop the moment it breaks.

```java
import io.github.sedislab.sentil.OnlineMonitor;
import io.github.sedislab.sentil.Robustness;
import java.util.Collections;

try (OnlineMonitor monitor = OnlineMonitor.create("always[0, 10] (x > -0.9)")) {
    for (int t = 0; t < 60; t++) {
        Robustness verdict = monitor.update(t, Collections.singletonMap("x", Math.sin(t * 0.3)));
        if (verdict.resolved() && !verdict.satisfied()) {
            System.out.printf("violated at t=%d, robustness=%.3f%n", t, verdict.value());
            break;
        }
    }
}
```

`satisfied` only carries a verdict once `resolved` is true; until then the monitor is still filling the window, so bound the horizon on a future-time operator.

## Probabilistic monitoring

A `P~p` operator asks whether a formula holds with probability at least (or at most) `p`. Register a noise model for each sensor; SENTIL lifts every reading into an ensemble, evaluates the formula on each, and returns the probability with a Wilson confidence interval.

```java
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.LiftingRegistry;
import io.github.sedislab.sentil.NoiseModel;
import io.github.sedislab.sentil.SmcConfig;
import io.github.sedislab.sentil.SmcResult;
import io.github.sedislab.sentil.Trace;

double[] times = new double[20];
double[] xs = new double[20];
for (int i = 0; i < 20; i++) {
    times[i] = i;
    xs[i] = 0.4 + 0.05 * i;
}
try (Trace trace = Trace.create(times);
        LiftingRegistry lifting = new LiftingRegistry();
        Formula phi = Formula.parse("P>=0.9 (always (x > 0))")) {
    trace.addSignal("x", xs);
    lifting.register("x", NoiseModel.gaussian(0.0, 0.3));

    SmcResult result = phi.check(trace, lifting, new SmcConfig().samples(5000));
    System.out.printf("probability %.3f, interval [%.3f, %.3f], holds %b%n",
            result.probability(), result.interval().lower(), result.interval().upper(),
            result.holds());
}
```

## Specifications

The premade library is on the Java side too: vetted specifications across ten domains (aerospace, automotive, controls, financial, industrial, medical, networking, power, robotics, UAV), each with a description, a citation, default parameters, and a deterministic and a probabilistic form. Build a formula straight from one.

```java
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.SpecBuilder;

try (SpecBuilder spec = new SpecBuilder("automotive/safe_following_distance");
        Formula phi = spec.withParam("rho", 1.0).buildFormula()) {  // the follower's reaction time
    // phi monitors gap, v_r, and v_f against the RSS safe-distance bound
}
```

List them with `SpecBuilder.available()`, or browse them under [`specifications/`](../specifications).

## Benchmarks

The jar carries the same engine as every other binding, so Java runs at the core's speed. These plots put Java and the Rust core against the baseline tools, from the same runs.

![Online streaming cost per sample: SENTIL (Java) among the bindings](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/streaming_java.png)

Per-sample streaming cost across the bindings, with the Rust core in front. The offline baselines have no online mode, so nothing else can stream a sample at a time.

![Offline cost over length: SENTIL (Java) and the core vs the baselines](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/scaling_java.png)

Offline cost over the trace length, Java and the core against RTAMT, MoonLight, and Banquo.

![Memory: SENTIL (Java) streams while the offline tools hold the whole trace](https://raw.githubusercontent.com/sedislab/SENTIL/main/benchmarks/results/memory_java.png)

Peak memory over the length of the stream.

The full set, including the dense-time, statistical model checking, rare-event, and synthesis benchmarks, is in [`benchmarks/`](../benchmarks), and all the results are in [`docs/CLAIMS.md`](../docs/CLAIMS.md).

## Install

### Package manager

From Maven Central, with Maven:

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

### Prebuilt release

To skip Maven, download the `sentil-0.3.0.jar` attached to the [GitHub release](https://github.com/sedislab/SENTIL/releases) and put it on the classpath directly; it carries the native library inside under `native/<os>-<arch>/`. The classpath separator differs by platform.

#### Linux and macOS

```sh
javac -cp sentil-0.3.0.jar MyMonitor.java
java -cp .:sentil-0.3.0.jar MyMonitor
```

#### Windows

```bat
javac -cp sentil-0.3.0.jar MyMonitor.java
java -cp .;sentil-0.3.0.jar MyMonitor
```

### Build from source

Building the jar compiles the core and a small JNI shim and packages both. You need a JDK (11 or newer) whose `include` directory carries `jni.h`, CMake, a C++ compiler, and a Rust toolchain.

```sh
git clone https://github.com/sedislab/SENTIL
cd SENTIL/sentil-java
mvn -DskipTests package
```

That runs `cargo build --release` for the core, builds the shim with CMake, and writes `target/sentil-0.3.0.jar` with the native library inside under `native/<os>-<arch>/`. It produces `libsentil.so` on Linux, `libsentil.dylib` on macOS, and `sentil.dll` on Windows, and the loader picks the right one at run time. To assemble one jar that runs everywhere, build on each platform and merge the per-platform `native/` trees, which is what the release workflow does.

## Contributing

Maven compiles the JNI shim against the bundled core and runs the tests:

```sh
cd sentil-java && mvn -B test
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Documentation

The [documentation site](https://sentil.pages.dev) carries the guides, the specification syntax, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial). The `examples/` directory ships one program per capability, the same set the other bindings carry, and each file's header shows how to compile and run it against the jar.

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