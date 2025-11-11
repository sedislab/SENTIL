using Test
using Sentil

@testset "specifications library" begin
    specs = available_specs()
    @test !isempty(specs)
    @test "aerospace/altitude_hold" in specs

    b = SpecBuilder("aerospace/altitude_hold")
    @test build_formula(b) isa Formula
    @test build_probabilistic_formula(b) isa Formula
    @test build_lifting_registry(b) isa LiftingRegistry
    @test build_deterministic(b) isa String && !isempty(build_deterministic(b))
    @test parameters_json(b) isa String
    @test available_variants(b) isa Vector{String}

    tuned = with_param(SpecBuilder("aerospace/altitude_hold"), "tolerance", 50.0)
    @test build_formula(tuned) isa Formula

    consumed = SpecBuilder("aerospace/altitude_hold")
    @test build_monitor(consumed) isa Monitor
    @test consumed.ptr == C_NULL

    err = try
        with_param(SpecBuilder("aerospace/altitude_hold"), "no_such_param", 1.0)
    catch e
        e
    end
    @test err isa SentilError && occursin("available", err.msg)

    s = smc_settings(SpecBuilder("aerospace/altitude_hold"))
    @test s === nothing || s isa SpecSmcSettings

    @testset "all build" begin
        for name in specs
            @test build_formula(SpecBuilder(name)) isa Formula
        end
    end
end
