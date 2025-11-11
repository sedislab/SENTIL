using Sentil

a = reshape([1.0], 1, 1)
b = reshape([1.0], 1, 1)
model = linear_model(a, b, [1.0], ["x"], 1.0, 3)
spec = formula("always (x > 0)")
bounds = Bounds([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])

result = synthesize(model, spec; bounds = bounds)
println("input: ", round.(result.input; digits = 4),
        "  robustness: ", result.robustness, "  holds: ", result.holds)

shield = SafetyFilter(bounds)
println("shielded: ", safe_input(shield, [2.0, 0.5, -3.0]))