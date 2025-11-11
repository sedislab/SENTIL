using Test
using Sentil

@testset "offline monitor" begin
    trace = Trace([0.0, 1.0, 2.0], "x", [3.0, 1.0, 2.0])
    m = Monitor("always[0, 2](x > 0)")
    @test robustness(m, trace) == 1.0
    @test robustness_signal(m, trace) == robustness_signal(formula("always[0,2](x>0)"), trace)
    @test time_mode(config(m)) == TimeMode.Discrete
    @test time_mode(config(Monitor("x > 0"; config = Config(time = TimeMode.Dense)))) == TimeMode.Dense
end

@testset "monitor update" begin
    m = Monitor("x > 0")
    r = update!(m, 0.0, Dict("x" => 2.5))
    @test r.value == 2.5 && r.satisfied
    @test symbol_index(m, "x") == 1
    @test symbol_index(m, "absent") === nothing
    reset!(m)
    @test update_packed!(m, 0.0, [4.0]).value == 4.0
end

@testset "online monitor" begin
    m = OnlineMonitor("x > 0")
    @test variable_count(m) == 1
    trace = Trace([0.0, 1.0, 2.0, 3.0], "x", [3.0, 1.0, 2.0, 4.0])
    verdicts = run!(m, trace)
    @test [v.value for v in verdicts] == [3.0, 1.0, 2.0, 4.0]
end

@testset "multi monitor" begin
    mm = MultiMonitor()
    add!(mm, "pos", "x > 0")
    add!(mm, "big", formula("x > 5"))
    @test ids(mm) == ["pos", "big"]
    v = update!(mm, 0.0, Dict("x" => 3.0))
    @test v["pos"].satisfied && !v["big"].satisfied
    @test remove!(mm, "big") && length(mm) == 1
end

@testset "formula bank" begin
    b = FormulaBank()
    add!(b, "a", "x > 0")
    add!(b, "b", "always[0,2](x > 1)")
    trace = Trace([0.0, 1.0, 2.0], "x", [3.0, 1.0, 2.0])
    r = robustness(b, trace)
    @test r["a"] == 3.0 && r["b"] == 0.0
    bad = FormulaBank()
    add!(bad, "u", "y > 0")
    @test_throws SentilError robustness(bad, trace)
end
