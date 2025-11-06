# Shared by every opaque handle. Each handle is a mutable struct holding a raw pointer;
# a finalizer frees it, and close! frees it eagerly. The pointer is null once freed.

"""Release a handle's native resources now instead of waiting for the garbage collector."""
function close! end

export close!

function _ptr(h)
    h.ptr == C_NULL &&
        throw(EvaluationError(SENTIL_ERR_NULL_POINTER, "this handle was already closed"))
    return h.ptr
end

function _consume!(h)
    p = _ptr(h)
    h.ptr = C_NULL
    return p
end

function _consume_all!(handles...)
    ptrs = map(_ptr, handles)
    for h in handles
        h.ptr = C_NULL
    end
    return ptrs
end

function _take_string(ptr::Ptr{UInt8})
    ptr == C_NULL && return ""
    s = unsafe_string(ptr)
    ccall((:sentil_free_string, libsentil[]), Cvoid, (Ptr{UInt8},), ptr)
    return s
end

function _take_string_array(ptr::Ptr{Ptr{UInt8}}, count::Integer)
    ptr == C_NULL && return String[]
    out = Vector{String}(undef, count)
    for i in 1:count
        out[i] = unsafe_string(unsafe_load(ptr, i))
    end
    ccall((:sentil_free_string_array, libsentil[]), Cvoid,
          (Ptr{Ptr{UInt8}}, Csize_t), ptr, count)
    return out
end

function _take_doubles(ptr::Ptr{Float64}, count::Integer)
    ptr == C_NULL && return Float64[]
    out = Vector{Float64}(undef, count)
    unsafe_copyto!(pointer(out), ptr, count)
    ccall((:sentil_free_doubles, libsentil[]), Cvoid, (Ptr{Float64}, Csize_t), ptr, count)
    return out
end

function _copy_doubles(ptr::Ptr{Float64}, count::Integer)
    ptr == C_NULL && return Float64[]
    out = Vector{Float64}(undef, count)
    unsafe_copyto!(pointer(out), ptr, count)
    return out
end

function _take_intervals(ptr::Ptr{Interval}, count::Integer)
    ptr == C_NULL && return Interval[]
    out = Vector{Interval}(undef, count)
    unsafe_copyto!(pointer(out), ptr, count)
    ccall((:sentil_free_intervals, libsentil[]), Cvoid, (Ptr{Interval}, Csize_t), ptr, count)
    return out
end

function _take_samples(ptr::Ptr{Sample}, count::Integer)
    ptr == C_NULL && return Sample[]
    out = Vector{Sample}(undef, count)
    unsafe_copyto!(pointer(out), ptr, count)
    ccall((:sentil_free_samples, libsentil[]), Cvoid, (Ptr{Sample}, Csize_t), ptr, count)
    return out
end