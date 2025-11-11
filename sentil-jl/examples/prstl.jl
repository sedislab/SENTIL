using Sentil

trace = Trace(collect(0.0:1.0:19.0), "x", [0.4 + 0.05 * i for i in 0:19])
lifting = LiftingRegistry()
register_noise!(lifting, "x", gaussian(0.0, 0.3))

phi = formula("P>=0.9 (always (x > 0))")
result = check(phi, trace, lifting; config = SmcConfig(samples = 5000))

println("probability ", round(result.probability; digits = 3),
        ", interval [", round(result.interval.lower; digits = 3),
        ", ", round(result.interval.upper; digits = 3),
        "], holds ", result.holds)