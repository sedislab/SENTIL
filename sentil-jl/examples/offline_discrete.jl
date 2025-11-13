using Sentil

trace = Trace([0.0, 1.0, 2.0, 3.0, 4.0], "speed", [12.0, 9.0, 7.0, 4.0, 6.0])
phi = formula("always (speed > 5)")

println("robustness: ", robustness(phi, trace))
println("per sample: ", robustness_signal(phi, trace))

for span in violations(phi, trace)
    println("violated on [", span.start, ", ", span.stop, "]")
end