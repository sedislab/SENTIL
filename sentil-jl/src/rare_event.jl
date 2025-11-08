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