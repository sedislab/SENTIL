using Test
using Sentil

@testset "Sentil.jl" begin
    include("test_oracle.jl")

    @testset "engines smoke" begin
        # Deterministic STL.
        trace = Trace([0.0, 1.0, 2.0], "x", [3.0, 1.0, 2.0])
        @test robustness(formula("always[0, 2](x > 0)"), trace) == 1.0
        @test robustness_signal(formula("x > 0"), trace) == [3.0, 1.0, 2.0]

        # PrSTL statistical monitoring.
        lifting = LiftingRegistry()
        register_noise!(lifting, "x", gaussian(0.0, 0.3))
        steady = Trace(collect(0.0:1.0:20.0), "x", fill(2.0, 21))
        @test check(formula("P>=0.5 (always (x > 0))"), steady, lifting).holds

        # Synthesis.
        a = reshape([1.0], 1, 1)
        model = linear_model(a, a, [0.0], ["x"], 1.0, 10)
        result = synthesize(model, formula("eventually[0, 10](x > 5)"); backend = Backend.Gradient)
        @test result.holds

        # The specifications library is present.
        @test !isempty(available_specs())
        @test build_formula(SpecBuilder(first(available_specs()))) isa Formula
    end
end
