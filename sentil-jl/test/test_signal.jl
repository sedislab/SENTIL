using Test
using Sentil

@testset "trace" begin
    t = Trace([0.0, 1.0, 2.0], "x", [3.0, 1.0, 2.0])
    @test length(t) == 3 && !isempty(t)
    @test times(t) == [0.0, 1.0, 2.0]
    @test t["x"] == [3.0, 1.0, 2.0]
    @test signal(t, "absent") === nothing
    add_signal!(t, "y", [0.0, 5.0, 4.0])
    @test variables(t) == ["x", "y"]
    @test indexed_trace(4) |> times == [0.0, 1.0, 2.0, 3.0]
    @test signal(parse_trace("time,x\n0,3\n1,1\n"), "x") == [3.0, 1.0]
end

@testset "resample" begin
    t = Trace([0.0, 2.0], "x", [0.0, 10.0])
    @test signal(resample(t, [0.0, 1.0, 2.0]), "x") == [0.0, 5.0, 10.0]
    p = prepare(t)
    @test signal(resample(p, [1.0]), "x") == [5.0]
end

@testset "ring buffer" begin
    b = RingBuffer(3)
    @test capacity(b) == 3 && isempty(b)
    push!(b, 0.0, 10.0)
    push!(b, 1.0, 20.0)
    push!(b, 2.0, 30.0)
    @test is_full(b) && front(b).value == 10.0 && back(b).value == 30.0
    @test b[1].value == 10.0
    evicted = push!(b, 3.0, 40.0)
    @test evicted.value == 10.0
    @test mean(b) == 30.0 && minimum(b) == 20.0 && maximum(b) == 40.0
    @test time_range(b) == (1.0, 3.0)
    @test length(between(b, 1.5, 3.0)) == 2
    clear!(b)
    @test isempty(b) && front(b) === nothing && mean(b) === nothing
end