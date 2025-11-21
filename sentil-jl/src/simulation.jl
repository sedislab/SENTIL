mutable struct SimExpr
    ptr::Ptr{Cvoid}
    function SimExpr(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        e = new(ptr)
        finalizer(_destroy, e)
        return e
    end
end

function _destroy(e::SimExpr)
    if e.ptr != C_NULL
        ccall((:sentil_sim_expr_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), e.ptr)
        e.ptr = C_NULL
    end
end

close!(e::SimExpr) = _destroy(e)

"""The previous value of variable `i` (1-based)."""
sim_prev(i::Integer) =
    SimExpr(ccall((:sentil_sim_expr_prev, libsentil[]), Ptr{Cvoid}, (Csize_t,), i - 1))
"""The current time."""
sim_time() = SimExpr(ccall((:sentil_sim_expr_time, libsentil[]), Ptr{Cvoid}, ()))
"""A constant term."""
sim_const(value::Real) =
    SimExpr(ccall((:sentil_sim_expr_const, libsentil[]), Ptr{Cvoid}, (Cdouble,), value))
"""A draw from noise source `i` (1-based)."""
sim_noise(i::Integer) =
    SimExpr(ccall((:sentil_sim_expr_noise, libsentil[]), Ptr{Cvoid}, (Csize_t,), i - 1))

for (op, c) in ((:+, :sentil_sim_expr_add), (:-, :sentil_sim_expr_sub),
                (:*, :sentil_sim_expr_mul), (:/, :sentil_sim_expr_div))
    @eval begin
        function Base.$op(a::SimExpr, b::SimExpr)
            l, r = _consume_all!(a, b)
            SimExpr(ccall(($(QuoteNode(c)), libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid}, Ptr{Cvoid}), l, r))
        end
        Base.$op(a::SimExpr, b::Real) = $op(a, sim_const(b))
        Base.$op(a::Real, b::SimExpr) = $op(sim_const(a), b)
    end
end

function _sim_call(name::AbstractString, args::SimExpr...)
    ptrs = collect(Ptr{Cvoid}, _consume_all!(args...))
    SimExpr(ccall((:sentil_sim_expr_call, libsentil[]), Ptr{Cvoid},
                  (Cstring, Ptr{Ptr{Cvoid}}, Csize_t), name, ptrs, length(ptrs)))
end

for fn in (:abs, :sin, :cos, :tan, :sqrt, :exp, :log, :floor, :ceil)
    @eval Base.$fn(e::SimExpr) = _sim_call($(string(fn)), e)
end

Base.min(a::SimExpr, b::SimExpr) = _sim_call("min", a, b)
Base.min(a::SimExpr, b::Real) = _sim_call("min", a, sim_const(b))
Base.min(a::Real, b::SimExpr) = _sim_call("min", sim_const(a), b)
Base.max(a::SimExpr, b::SimExpr) = _sim_call("max", a, b)
Base.max(a::SimExpr, b::Real) = _sim_call("max", a, sim_const(b))
Base.max(a::Real, b::SimExpr) = _sim_call("max", sim_const(a), b)

export SimExpr, sim_prev, sim_time, sim_const, sim_noise

mutable struct StochasticSystem
    ptr::Ptr{Cvoid}
    state::Any
    function StochasticSystem(ptr::Ptr{Cvoid}, state = nothing)
        ptr == C_NULL && _raise_last()
        s = new(ptr, state)
        finalizer(_destroy, s)
        return s
    end
end

function _destroy(s::StochasticSystem)
    if s.ptr != C_NULL
        ccall((:sentil_stochastic_system_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), s.ptr)
        s.ptr = C_NULL
    end
end

close!(s::StochasticSystem) = _destroy(s)

"""One simulated trajectory of the system from a seed."""
function simulate(s::StochasticSystem; seed::Integer = 42)
    ptr = GC.@preserve s ccall((:sentil_stochastic_system_simulate, libsentil[]), Ptr{Cvoid},
                               (Ptr{Cvoid}, UInt64), _ptr(s), seed)
    _rethrow_callback(s.state)
    return Trace(ptr)
end

function variables(s::StochasticSystem)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_stochastic_system_variables, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(s), n)
    return _take_string_array(ptr, n[])
end

dt(s::StochasticSystem) = ccall((:sentil_stochastic_system_dt, libsentil[]), Cdouble, (Ptr{Cvoid},), _ptr(s))
horizon(s::StochasticSystem) =
    Int(ccall((:sentil_stochastic_system_horizon, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(s)))

_rethrow_callback(::Any) = nothing

export StochasticSystem, simulate, variables, dt, horizon

mutable struct SimModel
    ptr::Ptr{Cvoid}
    function SimModel(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        m = new(ptr)
        finalizer(_destroy, m)
        return m
    end
end

function _destroy(m::SimModel)
    if m.ptr != C_NULL
        ccall((:sentil_sim_model_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), m.ptr)
        m.ptr = C_NULL
    end
end

close!(m::SimModel) = _destroy(m)

"""A declarative stochastic model with an init and an advance expression per variable."""
function SimModel(variables, dt::Real, horizon::Integer, init::AbstractVector{SimExpr},
                  advance::AbstractVector{SimExpr}, noise::AbstractVector{NoiseModel})
    names = String[String(v) for v in variables]
    init_p = Ptr{Cvoid}[_ptr(e) for e in init]
    adv_p = Ptr{Cvoid}[_ptr(e) for e in advance]
    noise_p = Ptr{Cvoid}[_ptr(m) for m in noise]
    for h in Iterators.flatten((init, advance, noise))
        h.ptr = C_NULL
    end
    SimModel(ccall((:sentil_sim_model_create, libsentil[]), Ptr{Cvoid},
                   (Ptr{Cstring}, Csize_t, Cdouble, Csize_t, Ptr{Ptr{Cvoid}}, Csize_t,
                    Ptr{Ptr{Cvoid}}, Csize_t, Ptr{Ptr{Cvoid}}, Csize_t),
                   names, length(names), dt, horizon, init_p, length(init_p),
                   adv_p, length(adv_p), noise_p, length(noise_p)))
end

"""One simulated trajectory of the model from a seed."""
simulate(m::SimModel; seed::Integer = 42) =
    Trace(ccall((:sentil_sim_model_simulate, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid}, UInt64), _ptr(m), seed))

function variables(m::SimModel)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_sim_model_variables, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(m), n)
    return _take_string_array(ptr, n[])
end

dt(m::SimModel) = ccall((:sentil_sim_model_dt, libsentil[]), Cdouble, (Ptr{Cvoid},), _ptr(m))
horizon(m::SimModel) = Int(ccall((:sentil_sim_model_horizon, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(m)))

"""Convert the model into a `StochasticSystem` the engine can sample in parallel."""
to_stochastic_system(m::SimModel) =
    StochasticSystem(ccall((:sentil_sim_model_to_stochastic_system, libsentil[]), Ptr{Cvoid},
                           (Ptr{Cvoid},), _ptr(m)))

export SimModel, to_stochastic_system

mutable struct _SystemBox
    init::Any
    step::Any
    err::Union{Nothing,Exception}
end

# Mirrors sentil_system_callbacks_t.
struct _SystemCallbacks
    userdata::Ptr{Cvoid}
    init::Ptr{Cvoid}
    step::Ptr{Cvoid}
end

function _system_init_trampoline(ud::Ptr{Cvoid}, seed::UInt64, out_state::Ptr{Float64}, n::Csize_t)::Cvoid
    box = unsafe_pointer_to_objref(ud)::_SystemBox
    try
        state = box.init(seed)
        for i in 1:Int(n)
            unsafe_store!(out_state, Float64(state[i]), i)
        end
    catch e
        box.err === nothing && (box.err = e)
    end
    return nothing
end

function _system_step_trampoline(ud::Ptr{Cvoid}, prev::Ptr{Float64}, n::Csize_t,
                                 time::Cdouble, seed::UInt64, out_state::Ptr{Float64})::Cvoid
    box = unsafe_pointer_to_objref(ud)::_SystemBox
    try
        prev_state = Float64[unsafe_load(prev, i) for i in 1:Int(n)]
        state = box.step(prev_state, time, seed)
        for i in 1:Int(n)
            unsafe_store!(out_state, Float64(state[i]), i)
        end
    catch e
        box.err === nothing && (box.err = e)
    end
    return nothing
end

const _C_SYSTEM_INIT = Ref{Ptr{Cvoid}}(C_NULL)
const _C_SYSTEM_STEP = Ref{Ptr{Cvoid}}(C_NULL)

_rethrow_callback(box::_SystemBox) = (box.err === nothing || throw(box.err))

"""A stochastic system whose dynamics are the host callbacks `init(seed)` and `step(prev, time, seed)`."""
function StochasticSystem(variables, dt::Real, horizon::Integer; init, step)
    names = String[String(v) for v in variables]
    box = _SystemBox(init, step, nothing)
    callbacks = _SystemCallbacks(pointer_from_objref(box), _C_SYSTEM_INIT[], _C_SYSTEM_STEP[])
    StochasticSystem(ccall((:sentil_stochastic_system_create, libsentil[]), Ptr{Cvoid},
                           (Ptr{Cstring}, Csize_t, Cdouble, Csize_t, _SystemCallbacks),
                           names, length(names), dt, horizon, callbacks), box)
end