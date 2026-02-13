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
    return GC.@preserve t begin
        ptr = ccall((:sentil_trace_times, libsentil[]), Ptr{Float64},
                    (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(t), n)
        _copy_doubles(ptr, n[])
    end
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
    return GC.@preserve t begin
        ptr = ccall((:sentil_trace_signal, libsentil[]), Ptr{Float64},
                    (Ptr{Cvoid}, Cstring, Ptr{Csize_t}), _ptr(t), name, n)
        ptr == C_NULL && return nothing
        _copy_doubles(ptr, n[])
    end
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

mutable struct RingBuffer
    ptr::Ptr{Cvoid}
    function RingBuffer(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        b = new(ptr)
        finalizer(_destroy, b)
        return b
    end
end

function _destroy(b::RingBuffer)
    if b.ptr != C_NULL
        ccall((:sentil_ring_buffer_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), b.ptr)
        b.ptr = C_NULL
    end
end

close!(b::RingBuffer) = _destroy(b)

"""A ring buffer holding the most recent `capacity` timed samples."""
RingBuffer(capacity::Integer) =
    RingBuffer(ccall((:sentil_ring_buffer_create, libsentil[]), Ptr{Cvoid}, (Csize_t,), capacity))

_sample(s::Sample) = s.found ? s : nothing

"""Append a sample, returning the evicted oldest sample when the buffer is full."""
function Base.push!(b::RingBuffer, time::Real, value::Real)
    evicted = Ref(Sample(false, 0.0, 0.0))
    check_error(ccall((:sentil_ring_buffer_push, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cdouble, Cdouble, Ptr{Sample}), _ptr(b), time, value, evicted))
    return _sample(evicted[])
end

Base.length(b::RingBuffer) =
    Int(ccall((:sentil_ring_buffer_len, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(b)))
Base.isempty(b::RingBuffer) =
    ccall((:sentil_ring_buffer_is_empty, libsentil[]), Bool, (Ptr{Cvoid},), _ptr(b))

"""The number of samples the buffer can hold."""
capacity(b::RingBuffer) =
    Int(ccall((:sentil_ring_buffer_capacity, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(b)))

"""Whether the buffer is at capacity."""
is_full(b::RingBuffer) =
    ccall((:sentil_ring_buffer_is_full, libsentil[]), Bool, (Ptr{Cvoid},), _ptr(b))

"""Drop every sample."""
clear!(b::RingBuffer) =
    (ccall((:sentil_ring_buffer_clear, libsentil[]), Cvoid, (Ptr{Cvoid},), _ptr(b)); b)

"""The oldest sample, or `nothing` when empty."""
front(b::RingBuffer) =
    _sample(ccall((:sentil_ring_buffer_front, libsentil[]), Sample, (Ptr{Cvoid},), _ptr(b)))

"""The newest sample, or `nothing` when empty."""
back(b::RingBuffer) =
    _sample(ccall((:sentil_ring_buffer_back, libsentil[]), Sample, (Ptr{Cvoid},), _ptr(b)))

"""The i-th sample, where 1 is the oldest."""
function Base.getindex(b::RingBuffer, i::Integer)
    1 <= i <= typemax(Csize_t) || throw(BoundsError(b, i))
    s = ccall((:sentil_ring_buffer_get, libsentil[]), Sample, (Ptr{Cvoid}, Csize_t), _ptr(b), i - 1)
    s.found || throw(BoundsError(b, i))
    return s
end

"""Remove and return the oldest sample, or `nothing` when empty."""
pop_front!(b::RingBuffer) =
    _sample(ccall((:sentil_ring_buffer_pop_front, libsentil[]), Sample, (Ptr{Cvoid},), _ptr(b)))

"""Remove and return the newest sample, or `nothing` when empty."""
pop_back!(b::RingBuffer) =
    _sample(ccall((:sentil_ring_buffer_pop_back, libsentil[]), Sample, (Ptr{Cvoid},), _ptr(b)))

"""The buffered sample whose time is nearest `time`, or `nothing` when empty."""
closest_to_time(b::RingBuffer, time::Real) =
    _sample(ccall((:sentil_ring_buffer_closest_to_time, libsentil[]), Sample,
                  (Ptr{Cvoid}, Cdouble), _ptr(b), time))

"""The value recorded at `time` within a small tolerance, or `nothing` if none is."""
function at_time(b::RingBuffer, time::Real)
    out = Ref{Float64}(0.0)
    ok = ccall((:sentil_ring_buffer_at_time, libsentil[]), Bool,
               (Ptr{Cvoid}, Cdouble, Ptr{Float64}), _ptr(b), time, out)
    return ok ? out[] : nothing
end

"""The earliest and latest times held as `(start, stop)`, or `nothing` when empty."""
function time_range(b::RingBuffer)
    lo = Ref{Float64}(0.0)
    hi = Ref{Float64}(0.0)
    ok = ccall((:sentil_ring_buffer_time_range, libsentil[]), Bool,
               (Ptr{Cvoid}, Ptr{Float64}, Ptr{Float64}), _ptr(b), lo, hi)
    return ok ? (lo[], hi[]) : nothing
end

"""The buffered samples whose time falls in `[start, stop]`."""
function between(b::RingBuffer, start::Real, stop::Real)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_ring_buffer_between, libsentil[]), Ptr{Sample},
                (Ptr{Cvoid}, Cdouble, Cdouble, Ptr{Csize_t}), _ptr(b), start, stop, n)
    ptr == C_NULL && _last_error_code() != SENTIL_OK && _raise_last()
    return _take_samples(ptr, n[])
end

for (jl, c) in ((:mean, :sentil_ring_buffer_mean), (:var, :sentil_ring_buffer_variance),
                (:std, :sentil_ring_buffer_std_dev), (:minimum, :sentil_ring_buffer_min),
                (:maximum, :sentil_ring_buffer_max))
    @eval function $jl(b::RingBuffer)
        out = Ref{Float64}(0.0)
        ok = ccall(($(QuoteNode(c)), libsentil[]), Bool,
                   (Ptr{Cvoid}, Ptr{Float64}), _ptr(b), out)
        return ok ? out[] : nothing
    end
end

"""Recompute the running mean and variance from the held samples."""
recompute_statistics!(b::RingBuffer) =
    (ccall((:sentil_ring_buffer_recompute_statistics, libsentil[]), Cvoid, (Ptr{Cvoid},), _ptr(b)); b)

export RingBuffer, capacity, is_full, clear!, front, back, pop_front!, pop_back!
export closest_to_time, at_time, time_range, between
export mean, var, std, recompute_statistics!