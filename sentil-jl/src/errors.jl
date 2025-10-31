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

function _last_error_message()
    n = ccall((:sentil_get_last_error_message, libsentil[]), Csize_t,
              (Ptr{UInt8}, Csize_t), C_NULL, 0)
    n == 0 && return ""
    buf = Vector{UInt8}(undef, n)
    ccall((:sentil_get_last_error_message, libsentil[]), Csize_t,
          (Ptr{UInt8}, Csize_t), buf, n)
    return GC.@preserve buf unsafe_string(pointer(buf))
end

_last_error_code() =
    SentilErrorCode(ccall((:sentil_get_last_error_code, libsentil[]), Int32, ()))

function _error(code::SentilErrorCode, msg::AbstractString)
    if code == SENTIL_ERR_PARSE
        ParseError(code, msg)
    elseif code == SENTIL_ERR_UNKNOWN_VARIABLE || code == SENTIL_ERR_NOT_PROBABILISTIC ||
           code == SENTIL_ERR_UNSUPPORTED
        SemanticError(code, msg)
    else
        EvaluationError(code, msg)
    end
end

function check_error(code::Integer)
    c = SentilErrorCode(code)
    c == SENTIL_OK && return nothing
    throw(_error(c, _last_error_message()))
end

function _raise_last()
    code = _last_error_code()
    throw(_error(code == SENTIL_OK ? SENTIL_ERR_PANIC : code, _last_error_message()))
end