"""A time grid paired with one or more named signals."""
mutable struct Trace
    ptr::Ptr{Cvoid}
    function Trace(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        t = new(ptr)
        finalizer(_destroy, t)
        return t
    end
end

function _destroy(t::Trace)
    if t.ptr != C_NULL
        ccall((:sentil_trace_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), t.ptr)
        t.ptr = C_NULL
    end
end

close!(t::Trace) = _destroy(t)

"""A trace over `times` with no signals yet."""
function Trace(times::AbstractVector{<:Real})
    t = convert(Vector{Float64}, times)
    Trace(ccall((:sentil_trace_create, libsentil[]), Ptr{Cvoid},
                (Ptr{Float64}, Csize_t), t, length(t)))
end

"""A trace over `times` carrying one signal."""
function Trace(times::AbstractVector{<:Real}, name::AbstractString, values::AbstractVector{<:Real})
    tt = convert(Vector{Float64}, times)
    vv = convert(Vector{Float64}, values)
    Trace(ccall((:sentil_trace_from_signal, libsentil[]), Ptr{Cvoid},
                (Ptr{Float64}, Csize_t, Cstring, Ptr{Float64}, Csize_t),
                tt, length(tt), name, vv, length(vv)))
end

"""A trace over `times` carrying every signal in the `name => values` mapping."""
function Trace(times::AbstractVector{<:Real}, signals::AbstractDict)
    t = Trace(times)
    for (name, values) in signals
        add_signal!(t, name, values)
    end
    return t
end

"""A trace whose time grid is `0, 1, ..., len - 1`."""
indexed_trace(len::Integer) =
    Trace(ccall((:sentil_trace_indexed, libsentil[]), Ptr{Cvoid}, (Csize_t,), len))

"""Add a named signal whose length matches the time grid."""
function add_signal!(t::Trace, name::AbstractString, values::AbstractVector{<:Real})
    vv = convert(Vector{Float64}, values)
    check_error(ccall((:sentil_trace_add_signal, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cstring, Ptr{Float64}, Csize_t), _ptr(t), name, vv, length(vv)))
    return t
end

Base.length(t::Trace) = Int(ccall((:sentil_trace_len, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(t)))
Base.isempty(t::Trace) = ccall((:sentil_trace_is_empty, libsentil[]), Bool, (Ptr{Cvoid},), _ptr(t))

"""The time grid."""
function times(t::Trace)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_trace_times, libsentil[]), Ptr{Float64},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(t), n)
    return _copy_doubles(ptr, n[])
end

function variables(t::Trace)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_trace_variables, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(t), n)
    return _take_string_array(ptr, n[])
end

"""The values of a named signal, or `nothing` when the trace carries no such signal."""
function signal(t::Trace, name::AbstractString)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_trace_signal, libsentil[]), Ptr{Float64},
                (Ptr{Cvoid}, Cstring, Ptr{Csize_t}), _ptr(t), name, n)
    ptr == C_NULL && return nothing
    return _copy_doubles(ptr, n[])
end

function Base.getindex(t::Trace, name::AbstractString)
    s = signal(t, name)
    s === nothing &&
        throw(SemanticError(SENTIL_ERR_UNKNOWN_VARIABLE, "the trace has no signal named `$name`"))
    return s
end

Base.haskey(t::Trace, name::AbstractString) = signal(t, name) !== nothing

"""Resample onto a new time grid, interpolating between the trace's samples."""
function resample(t::Trace, times::AbstractVector{<:Real}; interp::Interpolation.T = Interpolation.Linear)
    tt = convert(Vector{Float64}, times)
    Trace(ccall((:sentil_trace_resample, libsentil[]), Ptr{Cvoid},
                (Ptr{Cvoid}, Ptr{Float64}, Csize_t, Int32), _ptr(t), tt, length(tt), Int32(interp)))
end

mutable struct PreparedTrace
    ptr::Ptr{Cvoid}
    function PreparedTrace(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        p = new(ptr)
        finalizer(_destroy, p)
        return p
    end
end

function _destroy(p::PreparedTrace)
    if p.ptr != C_NULL
        ccall((:sentil_prepared_trace_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), p.ptr)
        p.ptr = C_NULL
    end
end

close!(p::PreparedTrace) = _destroy(p)

"""Fix an interpolation so the trace can be resampled onto many grids cheaply."""
prepare(t::Trace; interp::Interpolation.T = Interpolation.Linear) =
    PreparedTrace(ccall((:sentil_trace_prepare, libsentil[]), Ptr{Cvoid},
                        (Ptr{Cvoid}, Int32), _ptr(t), Int32(interp)))

function resample(p::PreparedTrace, times::AbstractVector{<:Real})
    tt = convert(Vector{Float64}, times)
    Trace(ccall((:sentil_prepared_trace_resample, libsentil[]), Ptr{Cvoid},
                (Ptr{Cvoid}, Ptr{Float64}, Csize_t), _ptr(p), tt, length(tt)))
end

"""Read a trace from a file, taking the format from the `.csv` or `.tsv` extension."""
read_trace(path::AbstractString) =
    Trace(ccall((:sentil_trace_from_path, libsentil[]), Ptr{Cvoid}, (Cstring,), path))

"""Parse a trace from in-memory delimited text, `:csv` or `:tsv`."""
function parse_trace(text::AbstractString; format::Symbol = :csv)
    if format === :csv
        Trace(ccall((:sentil_trace_from_csv, libsentil[]), Ptr{Cvoid}, (Cstring,), text))
    elseif format === :tsv
        Trace(ccall((:sentil_trace_from_tsv, libsentil[]), Ptr{Cvoid}, (Cstring,), text))
    else
        throw(ArgumentError("format must be :csv or :tsv, got :$format"))
    end
end

export Trace, indexed_trace, add_signal!, times, signal, resample, prepare
export read_trace, parse_trace