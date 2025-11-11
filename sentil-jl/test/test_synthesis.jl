using Test
using Sentil

@testset "numerics" begin
    @test soft_min([1.0, 2.0, 3.0], 1000.0) ≈ 1.0 atol = 1e-2
    @test soft_max([1.0, 2.0, 3.0], 1000.0) ≈ 3.0 atol = 1e-2
    @test solve_spd([2.0 0.0; 0.0 2.0], [2.0, 4.0]) ≈ [1.0, 2.0]
    @test solve_qp([1.0 0.0; 0.0 1.0], [0.0, 0.0], [-1.0 0.0; 0.0 -1.0], [-1.0, -1.0]) ≈ [1.0, 1.0] atol = 1e-3
    values, _ = symmetric_eigen([1.0 0.0; 0.0 3.0])
    @test sort(values) ≈ [1.0, 3.0]
    b = Bounds([-1.0, -1.0], [1.0, 1.0])
    p = [2.0, -3.0]
    clamp!(b, p)
    @test p == [1.0, -1.0]
end

@testset "open-loop synthesis" begin
    a = reshape([1.0], 1, 1)
    model = linear_model(a, a, [0.0], ["x"], 1.0, 10)
    @test input_dimension(model) == 10
    result = synthesize(model, formula("eventually[0, 10](x > 5)"); backend = Backend.Gradient)
    @test result.holds && length(result.input) == 10
    bounded = synthesize(model, formula("eventually[0, 10](x > 5)");
                         bounds = Bounds(fill(-2.0, 10), fill(2.0, 10)), backend = Backend.Gradient)
    @test all(-2.0 - 1e-6 .<= bounded.input .<= 2.0 + 1e-6)
end

@testset "controller and safety filter" begin
    a = reshape([1.0], 1, 1)
    ctrl = Controller(linear_model(a, a, [0.0], ["x"], 1.0, 5), formula("always[0, 5](x < 10)"),
                      1, 1_000_000_000; bounds = Bounds([-2.0], [2.0]))
    u = control(ctrl, [0.0])
    @test length(u) == 1 && -2.0 - 1e-6 <= u[1] <= 2.0 + 1e-6
    sf = SafetyFilter(Bounds([-1.0], [1.0]))
    @test safe_input(sf, [5.0]) ≈ [1.0]
    @test safe_input(sf, [-5.0]; barriers = [([1.0], 0.5)])[1] >= 0.5 - 1e-6
end

@testset "witnesses and optimizers" begin
    a = reshape([1.0], 1, 1)
    bnds = Bounds(fill(-1.0, 5), fill(1.0, 5))
    w = falsify(formula("always[0, 5](x < 0.5)"), linear_model(a, a, [0.0], ["x"], 1.0, 5), bnds; restarts = 3)
    @test w isa Witness && w.trace isa Trace
    point, value = maximize([0.0]; max_iters = 500) do x
        (-(x[1] - 3.0)^2, [-2.0 * (x[1] - 3.0)])
    end
    @test point[1] ≈ 3.0 atol = 1e-2
    @test value ≈ 0.0 atol = 1e-3
    p2, _ = cma_es([0.0]; config = CmaConfig(seed = 1, max_generations = 300)) do x
        -(x[1] - 3.0)^2
    end
    @test p2[1] ≈ 3.0 atol = 2e-1
end
