using Test
using Sentil

@testset "formula building" begin
    x = variable("x")
    y = variable("y")
    pairs = [
        (variable("x") + 2 > 5, "x + 2 > 5"),
        (3 - variable("x") >= 0, "3 - x >= 0"),
        (variable("x") * variable("y") <= 10, "x * y <= 10"),
        (variable("x")^2 == 4, "x ^ 2 == 4"),
        (abs(variable("x")) < 1, "abs(x) < 1"),
        (min(variable("x"), variable("y")) != 0, "min(x, y) != 0"),
        (sqrt(variable("x")) > 2, "sqrt(x) > 2"),
        (!(variable("x") > 0), "not (x > 0)"),
        ((variable("x") > 0) & (variable("y") < 1), "(x > 0) and (y < 1)"),
        (implies(variable("x") > 0, variable("y") < 1), "(x > 0) -> (y < 1)"),
        (always(variable("x") > 0; lower = 0, upper = 10), "always[0, 10](x > 0)"),
        (eventually(variable("x") > 5; upper = 3), "eventually[0, 3](x > 5)"),
        (until(variable("x") > 0, variable("y") > 0; upper = 5), "(x > 0) until[0, 5] (y > 0)"),
        (probability(always(variable("x") > 0), ProbabilityOp.Ge, 0.9), "P>=0.9 (always (x > 0))"),
    ]
    for (built, text) in pairs
        @test to_json(built) == to_json(formula(text))
    end
end

@testset "formula introspection" begin
    f = formula("always[0, 10](x > 5 and y < 3)")
    @test depth(f) > 1
    @test is_temporal(f)
    @test !is_temporal(formula("x > 0"))
    @test variables(f) == ["x", "y"]
    @test to_json(from_json(Formula, to_json(f))) == to_json(f)
end

@testset "expression math vs parser" begin
    t = Trace([0.0, 1.0, 2.0], "x", [4.0, 1.0, 9.0])
    for (built, text) in [(sqrt(variable("x")) > 1, "sqrt(x) > 1"),
                          (variable("x") % 4 < 2, "x % 4 < 2"),
                          (pow(variable("x"), 2) > 5, "x ^ 2 > 5")]
        @test robustness_signal(built, t) == robustness_signal(formula(text), t)
    end
end