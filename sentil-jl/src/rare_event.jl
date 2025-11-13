"""Configuration for rare-event splitting."""
struct RareEventConfig
    particles::Csize_t
    margin::Float64
    seed::UInt64
end

RareEventConfig(; particles::Integer = 4096, margin::Real = 0.0, seed::Integer = 42) =
    RareEventConfig(particles, margin, seed)

"""Estimate a `P~p` formula's satisfaction over a stochastic system by multilevel splitting."""
function check_rare_event(f::Formula, system::StochasticSystem; config::RareEventConfig = RareEventConfig())
    cfg = Ref(config)
    out = Ref{RareEventResult}()
    code = GC.@preserve system ccall((:sentil_formula_check_rare_event, libsentil[]), Int32,
                                     (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{RareEventConfig}, Ptr{RareEventResult}),
                                     _ptr(f), _ptr(system), cfg, out)
    _rethrow_callback(system.state)
    check_error(code)
    return out[]
end

"""The same over a monitor, with the monitor's configured defaults."""
function check_rare_event(m::Monitor, system::StochasticSystem)
    out = Ref{RareEventResult}()
    code = GC.@preserve system ccall((:sentil_monitor_check_rare, libsentil[]), Int32,
                                     (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{RareEventResult}), _ptr(m), _ptr(system), out)
    _rethrow_callback(system.state)
    check_error(code)
    return out[]
end

"""Rare-event splitting on the GPU over a declarative model."""
function check_rare_event_gpu(f::Formula, model::SimModel; config::RareEventConfig = RareEventConfig())
    cfg = Ref(config)
    out = Ref{GpuSplittingEstimate}()
    check_error(ccall((:sentil_formula_check_rare_event_gpu, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{RareEventConfig}, Ptr{GpuSplittingEstimate}),
                      _ptr(f), _ptr(model), cfg, out))
    return out[]
end

"""Whether a usable GPU device is present."""
gpu_available() = ccall((:sentil_gpu_is_available, libsentil[]), Bool, ())

export RareEventConfig, check_rare_event, check_rare_event_gpu, gpu_available

# The adaptive multilevel splitting interface over a host simulator with an opaque,
# fixed-size state. The state crosses the boundary as raw bytes, so it must be an isbits
# Julia type; the trampolines load and store it through a typed pointer. The callbacks
# run on several worker threads, so the first exception wins under a lock.
mutable struct _AmsBox
    State::DataType
    initial_state::Any
    step::Any
    is_terminal::Any
    score::Any
    err::Union{Nothing,Exception}
    lock::ReentrantLock
end

struct _AmsInterface
    state_size::Csize_t
    userdata::Ptr{Cvoid}
    initial_state::Ptr{Cvoid}
    step::Ptr{Cvoid}
    is_terminal::Ptr{Cvoid}
    score::Ptr{Cvoid}
end

_ams_capture(box, e) = Base.@lock box.lock (box.err === nothing && (box.err = e))

function _ams_initial_trampoline(ud::Ptr{Cvoid}, seed::UInt64, out_state::Ptr{Cvoid})::Cvoid
    box = unsafe_pointer_to_objref(ud)::_AmsBox
    try
        unsafe_store!(Ptr{box.State}(out_state), box.initial_state(seed))
    catch e
        _ams_capture(box, e)
    end
    return nothing
end

function _ams_step_trampoline(ud::Ptr{Cvoid}, state::Ptr{Cvoid}, seed::UInt64, out_state::Ptr{Cvoid})::Cvoid
    box = unsafe_pointer_to_objref(ud)::_AmsBox
    try
        s = unsafe_load(Ptr{box.State}(state))
        unsafe_store!(Ptr{box.State}(out_state), box.step(s, seed))
    catch e
        _ams_capture(box, e)
    end
    return nothing
end

function _ams_terminal_trampoline(ud::Ptr{Cvoid}, state::Ptr{Cvoid}, out_rare::Ptr{Bool})::Bool
    box = unsafe_pointer_to_objref(ud)::_AmsBox
    unsafe_store!(out_rare, false)
    try
        terminal, in_rare = box.is_terminal(unsafe_load(Ptr{box.State}(state)))
        unsafe_store!(out_rare, Bool(in_rare))
        return Bool(terminal)
    catch e
        _ams_capture(box, e)
        return true
    end
end

function _ams_score_trampoline(ud::Ptr{Cvoid}, state::Ptr{Cvoid})::Cdouble
    box = unsafe_pointer_to_objref(ud)::_AmsBox
    try
        return Float64(box.score(unsafe_load(Ptr{box.State}(state))))
    catch e
        _ams_capture(box, e)
        return NaN
    end
end

const _C_AMS_INIT = Ref{Ptr{Cvoid}}(C_NULL)
const _C_AMS_STEP = Ref{Ptr{Cvoid}}(C_NULL)
const _C_AMS_TERMINAL = Ref{Ptr{Cvoid}}(C_NULL)
const _C_AMS_SCORE = Ref{Ptr{Cvoid}}(C_NULL)

"""
    adaptive_multilevel_splitting(; state_type, initial_state, step, is_terminal, score,
                                  particles, target_score, max_steps, seed=42) -> RareEventEstimate

Estimate a rare-event probability over a host simulator. `state_type` must be an isbits
type. `initial_state(seed)` and `step(state, seed)` return a state; `is_terminal(state)`
returns `(terminal, in_rare_event)`; `score(state)` returns a real. The callbacks run on
worker threads, so keep them thread-safe and free of shared mutable state; the runtime
adopts those threads, so allocating and the garbage collector are safe inside them.
"""
function adaptive_multilevel_splitting(; state_type::Type, initial_state, step, is_terminal,
                                       score, particles::Integer, target_score::Real,
                                       max_steps::Integer, seed::Integer = 42)
    isbitstype(state_type) ||
        throw(EvaluationError(SENTIL_ERR_INVALID_CONFIG, "state_type must be an isbits type"))
    box = _AmsBox(state_type, initial_state, step, is_terminal, score, nothing, ReentrantLock())
    interface = _AmsInterface(sizeof(state_type), pointer_from_objref(box),
        _C_AMS_INIT[], _C_AMS_STEP[], _C_AMS_TERMINAL[], _C_AMS_SCORE[])
    out = Ref{RareEventEstimate}()
    code = GC.@preserve box ccall((:sentil_adaptive_multilevel_splitting, libsentil[]), Int32,
        (_AmsInterface, Csize_t, Cdouble, UInt64, UInt64, Ptr{RareEventEstimate}),
        interface, particles, target_score, max_steps, seed, out)
    box.err === nothing || throw(box.err)
    check_error(code)
    return out[]
end

export adaptive_multilevel_splitting