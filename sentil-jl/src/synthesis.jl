"""The soft semantics for differentiable robustness."""
struct SmoothConfig
    temperature::Float64
    kind::SoftKind.T
end

SmoothConfig(; temperature::Real = 10.0, kind::SoftKind.T = SoftKind.LogSumExp) =
    SmoothConfig(temperature, kind)

"""The differentiable robustness the synthesizers optimize."""
function smooth_robustness(f::Formula, trace::Trace; config::SmoothConfig = SmoothConfig())
    cfg = Ref(config)
    out = Ref{Float64}(0.0)
    check_error(ccall((:sentil_formula_smooth_robustness, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmoothConfig}, Ptr{Float64}),
                      _ptr(f), _ptr(trace), cfg, out))
    return out[]
end

"""The smooth robustness and its gradient with respect to every signal at every sample."""
function smooth_value_and_gradient(f::Formula, trace::Trace; config::SmoothConfig = SmoothConfig())
    vars = variables(f)
    nv = length(vars)
    ns = length(trace)
    cfg = Ref(config)
    value = Ref{Float64}(0.0)
    grad = Vector{Float64}(undef, nv * ns)
    check_error(ccall((:sentil_formula_smooth_value_and_gradient, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmoothConfig}, Ptr{Float64}, Ptr{Float64}, Csize_t, Csize_t),
                      _ptr(f), _ptr(trace), cfg, value, grad, nv, ns))
    # The gradient comes back variable-major.
    perVar = Dict{String,Vector{Float64}}()
    for (vi, name) in enumerate(vars)
        perVar[name] = grad[((vi - 1) * ns + 1):(vi * ns)]
    end
    return value[], perVar
end

export SmoothConfig, smooth_robustness, smooth_value_and_gradient

"""The soft minimum of `values` at the given temperature."""
function soft_min(values::AbstractVector{<:Real}, temperature::Real)
    v = convert(Vector{Float64}, values)
    ccall((:sentil_soft_min, libsentil[]), Cdouble, (Ptr{Float64}, Csize_t, Cdouble), v, length(v), temperature)
end

"""The soft maximum of `values` at the given temperature."""
function soft_max(values::AbstractVector{<:Real}, temperature::Real)
    v = convert(Vector{Float64}, values)
    ccall((:sentil_soft_max, libsentil[]), Cdouble, (Ptr{Float64}, Csize_t, Cdouble), v, length(v), temperature)
end

# C reads row-major, Julia stores column-major.
_rowmajor(m::AbstractMatrix{<:Real}) = collect(Float64, vec(permutedims(m)))

"""
    solve_qp(P, q, G, h; max_iters=200) -> Vector{Float64}

Minimize `½ uᵀ P u + qᵀ u` subject to `G u ≤ h`. `P` is `n×n`, `G` is `m×n`.
"""
function solve_qp(P::AbstractMatrix{<:Real}, q::AbstractVector{<:Real},
                  G::AbstractMatrix{<:Real}, h::AbstractVector{<:Real}; max_iters::Integer = 200)
    n = length(q)
    m = length(h)
    size(P) == (n, n) || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "P must be $n by $n"))
    size(G) == (m, n) || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "G must be $m by $n"))
    out = Vector{Float64}(undef, n)
    check_error(ccall((:sentil_solve_qp, libsentil[]), Int32,
                      (Ptr{Float64}, Csize_t, Ptr{Float64}, Ptr{Float64}, Csize_t, Ptr{Float64}, Csize_t, Ptr{Float64}),
                      _rowmajor(P), n, convert(Vector{Float64}, q), _rowmajor(G), m,
                      convert(Vector{Float64}, h), max_iters, out))
    return out
end

"""Solve `matrix x = rhs` for a symmetric positive-definite `matrix`."""
function solve_spd(matrix::AbstractMatrix{<:Real}, rhs::AbstractVector{<:Real})
    n = length(rhs)
    size(matrix) == (n, n) || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "matrix must be $n by $n"))
    out = Vector{Float64}(undef, n)
    check_error(ccall((:sentil_solve_spd, libsentil[]), Int32,
                      (Ptr{Float64}, Csize_t, Ptr{Float64}, Ptr{Float64}),
                      _rowmajor(matrix), n, convert(Vector{Float64}, rhs), out))
    return out
end

"""The eigenvalues and eigenvectors of a symmetric `matrix`, one eigenvector per row."""
function symmetric_eigen(matrix::AbstractMatrix{<:Real})
    n = size(matrix, 1)
    size(matrix) == (n, n) || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "matrix must be square"))
    values = Vector{Float64}(undef, n)
    vectors = Vector{Float64}(undef, n * n)
    check_error(ccall((:sentil_symmetric_eigen, libsentil[]), Int32,
                      (Ptr{Float64}, Csize_t, Ptr{Float64}, Ptr{Float64}),
                      _rowmajor(matrix), n, values, vectors))
    # vectors comes back row-major with one eigenvector per row.
    return values, permutedims(reshape(vectors, n, n))
end

export soft_min, soft_max, solve_qp, solve_spd, symmetric_eigen

mutable struct Bounds
    ptr::Ptr{Cvoid}
    function Bounds(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        b = new(ptr)
        finalizer(_destroy, b)
        return b
    end
end

function _destroy(b::Bounds)
    if b.ptr != C_NULL
        ccall((:sentil_bounds_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), b.ptr)
        b.ptr = C_NULL
    end
end

close!(b::Bounds) = _destroy(b)

"""A box constraint with the given per-coordinate lower and upper limits."""
function Bounds(lower::AbstractVector{<:Real}, upper::AbstractVector{<:Real})
    lo = convert(Vector{Float64}, lower)
    hi = convert(Vector{Float64}, upper)
    length(lo) == length(hi) ||
        throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG,
                              "lower has $(length(lo)) entries but upper has $(length(hi))"))
    Bounds(ccall((:sentil_bounds_create, libsentil[]), Ptr{Cvoid},
                 (Ptr{Float64}, Ptr{Float64}, Csize_t), lo, hi, length(lo)))
end

"""A box with no limits in any of `dimension` coordinates."""
unbounded_bounds(dimension::Integer) =
    Bounds(ccall((:sentil_bounds_unbounded, libsentil[]), Ptr{Cvoid}, (Csize_t,), dimension))

dimension(b::Bounds) = Int(ccall((:sentil_bounds_dimension, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(b)))

function lower(b::Bounds)
    out = Vector{Float64}(undef, dimension(b))
    ccall((:sentil_bounds_lower, libsentil[]), Cvoid, (Ptr{Cvoid}, Ptr{Float64}), _ptr(b), out)
    return out
end

function upper(b::Bounds)
    out = Vector{Float64}(undef, dimension(b))
    ccall((:sentil_bounds_upper, libsentil[]), Cvoid, (Ptr{Cvoid}, Ptr{Float64}), _ptr(b), out)
    return out
end

"""Project a point into the box in place."""
function Base.clamp!(b::Bounds, point::Vector{Float64})
    ccall((:sentil_bounds_clamp, libsentil[]), Cvoid,
          (Ptr{Cvoid}, Ptr{Float64}, Csize_t), _ptr(b), point, length(point))
    return point
end
Base.clamp!(b::Bounds, point::AbstractVector{<:Real}) = clamp!(b, convert(Vector{Float64}, point))

export Bounds, unbounded_bounds, dimension, lower, upper