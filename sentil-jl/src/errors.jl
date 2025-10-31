@enum SentilErrorCode::Int32 begin
    SENTIL_OK = 0
    SENTIL_ERR_NULL_POINTER = 1
    SENTIL_ERR_UTF8 = 2
    SENTIL_ERR_PARSE = 3
    SENTIL_ERR_UNKNOWN_VARIABLE = 4
    SENTIL_ERR_EVALUATION = 5
    SENTIL_ERR_TRACE = 6
    SENTIL_ERR_NOT_PROBABILISTIC = 7
    SENTIL_ERR_INVALID_NOISE_MODEL = 8
    SENTIL_ERR_INVALID_CONFIG = 9
    SENTIL_ERR_FIT = 10
    SENTIL_ERR_INGEST = 11
    SENTIL_ERR_SPLITTING = 12
    SENTIL_ERR_UNSUPPORTED = 13
    SENTIL_ERR_TRANSPILATION = 14
    SENTIL_ERR_GPU = 15
    SENTIL_ERR_JSON = 16
    SENTIL_ERR_PANIC = 17
end

export SentilErrorCode

"""Supertype of every error the library raises."""
abstract type SentilError <: Exception end

"""A formula or input that did not parse."""
struct ParseError <: SentilError
    code::SentilErrorCode
    msg::String
end

"""A well-formed input the engine cannot make sense of."""
struct SemanticError <: SentilError
    code::SentilErrorCode
    msg::String
end

"""A failure raised while evaluating or running."""
struct EvaluationError <: SentilError
    code::SentilErrorCode
    msg::String
end

Base.showerror(io::IO, e::SentilError) = print(io, nameof(typeof(e)), '(', e.code, "): ", e.msg)

export SentilError, ParseError, SemanticError, EvaluationError