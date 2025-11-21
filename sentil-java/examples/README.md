# Examples

Examples of simple SENTIL monitors. Build the jar once from the binding root, then compile and run an example against it:

```
mvn -o -DskipTests package
cd examples
javac -cp ../target/sentil-1.0.0.jar OfflineMonitoring.java
java -cp ../target/sentil-1.0.0.jar:. OfflineMonitoring
```

`OfflineMonitoring` evaluates a formula over a recorded trace in discrete and dense time. `OnlineStreaming` folds one sample at a time and reports the first violation. `Probabilistic` lifts a noisy sensor and estimates satisfaction with a confidence interval. `Synthesis` finds a control input that satisfies a spec, then shields a nominal input back into its bounds.