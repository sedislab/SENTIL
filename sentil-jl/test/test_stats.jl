using Test
using Sentil
using Random: seed!

@testset "confidence intervals" begin
    ci = wilson_interval(80, 100, 0.95)
    @test ci.lower < 0.8 < ci.upper && ci.level == 0.95
    @test clopper_pearson(80, 100, 0.95).lower < ci.lower
    @test z_score(0.95) ≈ 1.959963984540054 atol = 1e-9
    @test chernoff_hoeffding_samples(0.01, 0.05) > 0
    @test wilson_samples(0.01, 0.95) > 0
end

@testset "statistical model checking" begin
    lifting = LiftingRegistry()
    register_noise!(lifting, "x", gaussian(0.0, 0.3))
    trace = Trace(collect(0.0:1.0:20.0), "x", fill(2.0, 21))
    phi = formula("P>=0.8 (always (x > 0))")
    r = check(phi, trace, lifting)
    @test r.holds && r.probability > 0.9 && r.samples == 10000
    @test check_conservative(phi, trace, lifting) isa SmcResult
    _, dist = check_distribution(phi, trace, lifting)
    @test dist.count > 0
    @test_throws SemanticError check(formula("x > 0"), trace, lifting)
end

@testset "sequential testing" begin
    seed!(1)
    lifting = LiftingRegistry()
    register_noise!(lifting, "x", gaussian(0.0, 0.3))
    trace = Trace(collect(0.0:1.0:30.0), "x", fill(2.0, 31))
    phi = formula("P>=0.5 (always (x > 0))")
    @test check_sequential(phi, trace, lifting, SprtConfig(0.4, 0.9)).verdict == SprtVerdict.AcceptH1
    @test check_bayesian(phi, trace, lifting, BayesConfig(0.5)).verdict == BayesVerdict.Holds
    @test sequential_test(SprtConfig(0.3, 0.7)) do
        rand() < 0.9
    end.verdict == SprtVerdict.AcceptH1
    @test_throws ErrorException sequential_test(SprtConfig(0.3, 0.7)) do
        error("from the draw")
    end
end

@testset "noise models" begin
    @test mean(gaussian(2.0, 0.5)) == 2.0
    @test var(gaussian(2.0, 0.5)) ≈ 0.25
    @test residuals([1.0, 2.0], [2.0, 6.0]; interaction = NoiseInteraction.Multiplicative) == [2.0, 3.0]
    j = to_json(gaussian(1.0, 0.5))
    @test to_json(from_json(NoiseModel, j)) == j
    c1, c2 = gaussian(0.0, 1.0), gaussian(5.0, 1.0)
    @test mixture([0.5, 0.5], [c1, c2]) isa NoiseModel
    @test c1.ptr == C_NULL
end
