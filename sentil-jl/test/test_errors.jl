using Test
using Sentil

@testset "parse errors" begin
    @test_throws ParseError formula("always(((")
    @test_throws ParseError formula("x >")
    err = try
        formula("always(((")
    catch e
        e
    end
    @test err isa ParseError && occursin("column", err.msg)
end

@testset "semantic errors" begin
    trace = Trace([0.0, 1.0], "x", [1.0, 2.0])
    @test_throws SentilError robustness(formula("P>=0.5 (x > 0)"), trace)
    @test_throws SentilError robustness(formula("y > 0"), trace)
    lifting = LiftingRegistry()
    register_noise!(lifting, "x", gaussian(0.0, 0.1))
    @test_throws SemanticError check(formula("x > 0"), trace, lifting)
end

@testset "input errors" begin
    @test_throws SentilError Trace([0.0, 1.0, 2.0], "x", [1.0])
    @test_throws SemanticError probability(formula("x > 0"), ProbabilityOp.Ge, 1.5)
    @test_throws SentilError from_json(NoiseModel, "{not valid json")
end

@testset "use after close" begin
    f = formula("x > 0")
    close!(f)
    @test f.ptr == C_NULL
    close!(f)
    @test_throws EvaluationError depth(f)
end

@testset "bank names the failing id" begin
    bank = FormulaBank()
    add!(bank, "bad", "y > 0")
    err = try
        robustness(bank, Trace([0.0], "x", [1.0]))
    catch e
        e
    end
    @test err isa SentilError && occursin("bad", err.msg)
end