using Test
using Sentil
import JSON

const ORACLE_PATH = normpath(joinpath(@__DIR__, "..", "..", "benchmarks", "deterministic", "oracle.json"))

parse_token(t::AbstractString) =
    t == "inf" ? Inf : t == "-inf" ? -Inf : t == "nan" ? NaN : parse(Float64, t)

bit_equal(got, exp) = (isnan(got) && isnan(exp)) || got === exp

@testset "deterministic oracle" begin
    oracle = JSON.parsefile(ORACLE_PATH)
    cases = oracle["deterministic"]
    @test !isempty(cases)
    for case in cases
        @testset "$(case["id"])" begin
            trace = indexed_trace(Int(case["length"]))
            for sig in case["signals"]
                add_signal!(trace, sig["name"], Float64[parse_token(v) for v in sig["values"]])
            end
            got = robustness_signal(formula(case["formula"]), trace)
            expected = Float64[parse_token(v) for v in case["expected"]]
            @test length(got) == length(expected)
            @test all(bit_equal(g, e) for (g, e) in zip(got, expected))
        end
    end
end
