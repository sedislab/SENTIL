using Sentil

trace = Trace([0.0, 1.0, 2.0, 3.0, 4.0], "speed", [12.0, 9.0, 7.0, 4.0, 6.0])
phi = formula("always (speed > 5)")

println("dense robustness: ", robustness(phi, trace; dense = true))
println("dense per sample: ", robustness_signal(phi, trace; dense = true))

fine = resample(trace, collect(0.0:0.5:4.0))
println("speed at t=2.5: ", signal(fine, "speed")[6])