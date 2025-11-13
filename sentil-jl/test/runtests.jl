using Test
using Sentil

@testset "Sentil.jl" begin
    include("test_oracle.jl")
    include("test_formula.jl")
    include("test_signal.jl")
    include("test_monitor.jl")
    include("test_stats.jl")
    include("test_synthesis.jl")
    include("test_specs.jl")
    include("test_errors.jl")

    @testset "simulation and rare-event" begin
        @test version() == (1, 0, 0)   # the loaded core is the one this package binds
        model = SimModel(["x"], 0.1, 20, [sim_const(0.0)], [sim_prev(1) + sim_noise(1)], [gaussian(0.0, 0.1)])
        @test simulate(model; seed = 42) isa Trace
        sys = to_stochastic_system(model)
        report = check_rare_event(formula("P>=0.99 (always (x > -100))"), sys;
                                  config = RareEventConfig(particles = 256, seed = 1))
        @test report.simulations > 0
        @test gpu_available() isa Bool
    end
end