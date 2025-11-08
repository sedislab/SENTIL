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

mutable struct SystemModel
    ptr::Ptr{Cvoid}
    state::Any
    function SystemModel(ptr::Ptr{Cvoid}, state = nothing)
        ptr == C_NULL && _raise_last()
        m = new(ptr, state)
        finalizer(_destroy, m)
        return m
    end
end

function _destroy(m::SystemModel)
    if m.ptr != C_NULL
        ccall((:sentil_system_model_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), m.ptr)
        m.ptr = C_NULL
    end
end

close!(m::SystemModel) = _destroy(m)

"""
    linear_model(A, B, x0, variables, dt, horizon) -> SystemModel

A discrete-time linear model `x' = A x + B u` over the named variables. `A` is `n×n`,
`B` is `n×b`, and `x0` is the initial state of length `n`.
"""
function linear_model(A::AbstractMatrix{<:Real}, B::AbstractMatrix{<:Real},
                      x0::AbstractVector{<:Real}, variables, dt::Real, horizon::Integer)
    n = length(x0)
    size(A) == (n, n) || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "A must be $n by $n"))
    size(B, 1) == n || throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "B must have $n rows"))
    names = String[String(v) for v in variables]
    SystemModel(ccall((:sentil_linear_model_create, libsentil[]), Ptr{Cvoid},
                      (Ptr{Float64}, Csize_t, Ptr{Float64}, Csize_t, Ptr{Float64},
                       Ptr{Cstring}, Csize_t, Cdouble, Csize_t),
                      _rowmajor(A), n, _rowmajor(B), size(B, 2), convert(Vector{Float64}, x0),
                      names, length(names), dt, horizon))
end

"""The total number of input values the model takes over the horizon."""
input_dimension(m::SystemModel) =
    Int(ccall((:sentil_system_model_input_dimension, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(m)))

# Mirrors sentil_synthesis_result_t.
struct _SynthesisResult
    input::Ptr{Float64}
    input_len::Csize_t
    robustness::Float64
    holds::Bool
    backend::Backend.T
end

"""The outcome of `synthesize`."""
struct SynthesisResult
    input::Vector{Float64}
    robustness::Float64
    holds::Bool
    backend::Backend.T
end

# A NULL SmoothConfig pointer takes the engine default.
function _smooth_ref(smooth)
    smooth === nothing && return Ref{SmoothConfig}(), Ptr{SmoothConfig}(C_NULL)
    r = Ref(smooth)
    return r, Base.unsafe_convert(Ptr{SmoothConfig}, r)
end

"""Find an input sequence for the model that best satisfies the spec."""
function synthesize(model::SystemModel, spec::Formula; bounds = nothing, smooth = nothing,
                    backend::Backend.T = Backend.Auto, max_iters::Integer = 0, population::Integer = 0)
    bptr = bounds === nothing ? Ptr{Cvoid}(C_NULL) : _ptr(bounds)
    sref, sptr = _smooth_ref(smooth)
    out = Ref{_SynthesisResult}()
    code = GC.@preserve model sref ccall((:sentil_synthesize, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmoothConfig}, Csize_t, Int32, Csize_t, Ptr{_SynthesisResult}),
        _ptr(model), _ptr(spec), bptr, sptr, max_iters, Int32(backend), population, out)
    _rethrow_callback(model.state)
    check_error(code)
    r = out[]
    return SynthesisResult(_take_doubles(r.input, r.input_len), r.robustness, r.holds, r.backend)
end

export SystemModel, linear_model, input_dimension, SynthesisResult, synthesize

"""Settings for the CMA-ES search."""
struct CmaConfig
    population::Csize_t
    max_generations::Csize_t
    initial_step::Float64
    tol_step::Float64
    seed::UInt64
end

CmaConfig(; population::Integer = 0, max_generations::Integer = 300, initial_step::Real = 0.3,
          tol_step::Real = 1e-11, seed::Integer = 42) =
    CmaConfig(population, max_generations, initial_step, tol_step, seed)

export CmaConfig

mutable struct Controller
    ptr::Ptr{Cvoid}
    state::Any
    input_width::Int
    function Controller(ptr::Ptr{Cvoid}, state, input_width)
        ptr == C_NULL && _raise_last()
        c = new(ptr, state, input_width)
        finalizer(_destroy, c)
        return c
    end
end

function _destroy(c::Controller)
    if c.ptr != C_NULL
        ccall((:sentil_controller_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), c.ptr)
        c.ptr = C_NULL
    end
end

close!(c::Controller) = _destroy(c)

"""A controller that plans a short horizon each step and emits an input within the `budget_ns` deadline."""
function Controller(model::SystemModel, spec::Formula, input_width::Integer, budget_ns::Integer;
                    bounds = nothing, smooth = nothing)
    bptr = bounds === nothing ? Ptr{Cvoid}(C_NULL) : _ptr(bounds)
    sref, sptr = _smooth_ref(smooth)
    box = model.state
    mptr = _consume!(model)
    spptr = _consume!(spec)
    ptr = GC.@preserve sref ccall((:sentil_controller_create, libsentil[]), Ptr{Cvoid},
        (Ptr{Cvoid}, Ptr{Cvoid}, Csize_t, UInt64, Ptr{Cvoid}, Ptr{SmoothConfig}),
        mptr, spptr, input_width, budget_ns, bptr, sptr)
    Controller(ptr, box, Int(input_width))
end

"""Plan from the current state and return the control input."""
function control(c::Controller, state::AbstractVector{<:Real})
    s = convert(Vector{Float64}, state)
    out = Vector{Float64}(undef, c.input_width)
    code = GC.@preserve c ccall((:sentil_controller_control, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Float64}, Csize_t, Ptr{Float64}), _ptr(c), s, length(s), out)
    _rethrow_callback(c.state)
    check_error(code)
    return out
end

export Controller, control

mutable struct SafetyFilter
    ptr::Ptr{Cvoid}
    function SafetyFilter(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        f = new(ptr)
        finalizer(_destroy, f)
        return f
    end
end

function _destroy(f::SafetyFilter)
    if f.ptr != C_NULL
        ccall((:sentil_safety_filter_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), f.ptr)
        f.ptr = C_NULL
    end
end

close!(f::SafetyFilter) = _destroy(f)

"""A safety filter that keeps inputs inside a box, consuming the bounds."""
SafetyFilter(bounds::Bounds) =
    SafetyFilter(ccall((:sentil_safety_filter_create, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _consume!(bounds)))

"""
    safe_input(safety_filter, nominal; barriers=[]) -> Vector{Float64}

The input closest to `nominal` that satisfies the bounds and each barrier `(coeff, bound)`
meaning `coeff · u ≥ bound`. Each coeff has the same length as `nominal`. Named to avoid
clashing with `Base.filter`.
"""
function safe_input(sf::SafetyFilter, nominal::AbstractVector{<:Real};
                    barriers::AbstractVector = Tuple{Vector{Float64},Float64}[])
    nom = convert(Vector{Float64}, nominal)
    n = length(nom)
    m = length(barriers)
    a = Float64[]
    bvec = Float64[]
    for (coeff, bound) in barriers
        length(coeff) == n ||
            throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "each barrier coefficient must have length $n"))
        append!(a, Float64.(coeff))
        push!(bvec, Float64(bound))
    end
    out = Vector{Float64}(undef, n)
    check_error(ccall((:sentil_safety_filter_filter, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Float64}, Csize_t, Ptr{Float64}, Ptr{Float64}, Csize_t, Ptr{Float64}),
        _ptr(sf), nom, n, isempty(a) ? C_NULL : a, isempty(bvec) ? C_NULL : bvec, m, out))
    return out
end

export SafetyFilter, safe_input

mutable struct ChanceConstraint
    ptr::Ptr{Cvoid}
    function ChanceConstraint(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        c = new(ptr)
        finalizer(_destroy, c)
        return c
    end
end

function _destroy(c::ChanceConstraint)
    if c.ptr != C_NULL
        ccall((:sentil_chance_constraint_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), c.ptr)
        c.ptr = C_NULL
    end
end

close!(c::ChanceConstraint) = _destroy(c)

"""A constraint that the spec holds with at least `probability`, consuming the spec."""
ChanceConstraint(spec::Formula, probability::Real; confidence::Real = 0.0, tightening::Real = 0.0) =
    ChanceConstraint(ccall((:sentil_chance_constraint_create, libsentil[]), Ptr{Cvoid},
                           (Ptr{Cvoid}, Cdouble, Cdouble, Cdouble),
                           _consume!(spec), probability, confidence, tightening))

"""Validate the constraint over a stochastic system by sampling."""
function validate(cc::ChanceConstraint, system::StochasticSystem; samples::Integer = 1000, seed::Integer = 42)
    out = Ref{ChanceReport}()
    code = GC.@preserve system ccall((:sentil_chance_constraint_validate, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, UInt64, UInt64, Ptr{ChanceReport}), _ptr(cc), _ptr(system), samples, seed, out)
    _rethrow_callback(system.state)
    check_error(code)
    return out[]
end

export ChanceConstraint, validate

# Mirrors sentil_witness_t.
struct _Witness
    input::Ptr{Float64}
    input_len::Csize_t
    robustness::Float64
    trace::Ptr{Cvoid}
end

"""A witnessing run found by a search."""
struct Witness
    input::Vector{Float64}
    robustness::Float64
    trace::Trace
end

_witness(w::_Witness) = Witness(_take_doubles(w.input, w.input_len), w.robustness, Trace(w.trace))

"""Descend the smooth robustness from the model's initial state to find a witnessing run."""
function find_counterexample(f::Formula, model::SystemModel, bounds::Bounds;
                             max_iters::Integer = 200, smooth = nothing)
    sref, sptr = _smooth_ref(smooth)
    out = Ref{_Witness}()
    code = GC.@preserve model sref ccall((:sentil_formula_find_counterexample, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Csize_t, Ptr{SmoothConfig}, Ptr{_Witness}),
        _ptr(f), _ptr(model), _ptr(bounds), max_iters, sptr, out)
    _rethrow_callback(model.state)
    check_error(code)
    return _witness(out[])
end

"""Minimize the exact robustness with restarted CMA-ES to falsify the spec."""
function falsify(f::Formula, model::SystemModel, bounds::Bounds; config::CmaConfig = CmaConfig(), restarts::Integer = 1)
    out = Ref{_Witness}()
    code = GC.@preserve model ccall((:sentil_formula_falsify, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, CmaConfig, Csize_t, Ptr{_Witness}),
        _ptr(f), _ptr(model), _ptr(bounds), config, restarts, out)
    _rethrow_callback(model.state)
    check_error(code)
    return _witness(out[])
end

export Witness, find_counterexample, falsify

"""The smooth robustness of the trajectory the model rolls from `initial` under `input`, and its gradient."""
function smooth_gradient(f::Formula, model::SystemModel, initial::AbstractVector{<:Real},
                         input::AbstractVector{<:Real}; config::SmoothConfig = SmoothConfig())
    init = convert(Vector{Float64}, initial)
    inp = convert(Vector{Float64}, input)
    cfg = Ref(config)
    value = Ref{Float64}(0.0)
    grad = Vector{Float64}(undef, length(inp))
    code = GC.@preserve model ccall((:sentil_formula_smooth_gradient, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Float64}, Csize_t, Ptr{Float64}, Csize_t, Ptr{SmoothConfig}, Ptr{Float64}, Ptr{Float64}),
        _ptr(f), _ptr(model), init, length(init), inp, length(inp), cfg, value, grad)
    _rethrow_callback(model.state)
    check_error(code)
    return value[], grad
end

export smooth_gradient

mutable struct _GradientBox
    objective::Any
    err::Union{Nothing,Exception}
end

function _gradient_trampoline(ud::Ptr{Cvoid}, x::Ptr{Float64}, n::Csize_t,
                              out_value::Ptr{Float64}, out_gradient::Ptr{Float64})::Cvoid
    box = unsafe_pointer_to_objref(ud)::_GradientBox
    try
        xv = Float64[unsafe_load(x, i) for i in 1:Int(n)]
        value, grad = box.objective(xv)
        unsafe_store!(out_value, Float64(value))
        for i in 1:Int(n)
            unsafe_store!(out_gradient, Float64(grad[i]), i)
        end
    catch e
        box.err === nothing && (box.err = e)
    end
    return nothing
end

mutable struct _ObjectiveBox
    objective::Any
    err::Union{Nothing,Exception}
end

function _objective_trampoline(ud::Ptr{Cvoid}, x::Ptr{Float64}, n::Csize_t)::Cdouble
    box = unsafe_pointer_to_objref(ud)::_ObjectiveBox
    try
        xv = Float64[unsafe_load(x, i) for i in 1:Int(n)]
        return Float64(box.objective(xv))
    catch e
        box.err === nothing && (box.err = e)
        return NaN
    end
end

const _C_GRADIENT = Ref{Ptr{Cvoid}}(C_NULL)
const _C_OBJECTIVE = Ref{Ptr{Cvoid}}(C_NULL)

"""Maximize a differentiable objective by gradient ascent, where `objective(x)` returns `(value, gradient)`."""
function maximize(objective, start::AbstractVector{<:Real}; bounds = nothing, max_iters::Integer = 0)
    s = convert(Vector{Float64}, start)
    n = length(s)
    b = bounds === nothing ? unbounded_bounds(n) : bounds
    box = _GradientBox(objective, nothing)
    point = Vector{Float64}(undef, n)
    value = Ref{Float64}(0.0)
    code = GC.@preserve box b ccall((:sentil_maximize, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Float64}, Csize_t, Ptr{Cvoid}, Csize_t, Ptr{Float64}, Ptr{Float64}),
        _C_GRADIENT[], pointer_from_objref(box), s, n, _ptr(b), max_iters, point, value)
    box.err === nothing || throw(box.err)
    check_error(code)
    return point, value[]
end

"""Maximize a black-box objective with CMA-ES, where `objective(x)` returns a scalar."""
function cma_es(objective, start::AbstractVector{<:Real}; bounds = nothing, config::CmaConfig = CmaConfig())
    s = convert(Vector{Float64}, start)
    n = length(s)
    b = bounds === nothing ? unbounded_bounds(n) : bounds
    box = _ObjectiveBox(objective, nothing)
    point = Vector{Float64}(undef, n)
    value = Ref{Float64}(0.0)
    code = GC.@preserve box b ccall((:sentil_cma_es, libsentil[]), Int32,
        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Float64}, Csize_t, Ptr{Cvoid}, CmaConfig, Ptr{Float64}, Ptr{Float64}),
        _C_OBJECTIVE[], pointer_from_objref(box), s, n, _ptr(b), config, point, value)
    box.err === nothing || throw(box.err)
    check_error(code)
    return point, value[]
end

export maximize, cma_es