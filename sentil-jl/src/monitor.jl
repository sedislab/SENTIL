mutable struct Config
    ptr::Ptr{Cvoid}
    function Config(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        c = new(ptr)
        finalizer(_destroy, c)
        return c
    end
end

function _destroy(c::Config)
    if c.ptr != C_NULL
        ccall((:sentil_monitor_config_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), c.ptr)
        c.ptr = C_NULL
    end
end

close!(c::Config) = _destroy(c)

"""Monitoring configuration."""
function Config(; time::TimeMode.T = TimeMode.Discrete)
    c = Config(ccall((:sentil_monitor_config_create, libsentil[]), Ptr{Cvoid}, ()))
    check_error(ccall((:sentil_monitor_config_set_time, libsentil[]), Int32,
                      (Ptr{Cvoid}, Int32), _ptr(c), Int32(time)))
    return c
end

"""The time mode the config selects."""
time_mode(c::Config) =
    TimeMode.T(ccall((:sentil_monitor_config_time_mode, libsentil[]), Int32, (Ptr{Cvoid},), _ptr(c)))

export Config, time_mode

function _split_samples(samples::AbstractDict)
    names = Vector{String}(undef, length(samples))
    vals = Vector{Float64}(undef, length(samples))
    for (i, (k, v)) in enumerate(samples)
        names[i] = string(k)
        vals[i] = Float64(v)
    end
    return names, vals
end

mutable struct Monitor
    ptr::Ptr{Cvoid}
    function Monitor(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        m = new(ptr)
        finalizer(_destroy, m)
        return m
    end
end

function _destroy(m::Monitor)
    if m.ptr != C_NULL
        ccall((:sentil_monitor_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), m.ptr)
        m.ptr = C_NULL
    end
end

close!(m::Monitor) = _destroy(m)

"""A monitor for a formula given as a `Formula` or as text."""
Monitor(f::Formula; config::Config = Config()) =
    Monitor(ccall((:sentil_monitor_create, libsentil[]), Ptr{Cvoid},
                  (Ptr{Cvoid}, Ptr{Cvoid}), _consume!(f), _ptr(config)))

Monitor(text::AbstractString; config::Config = Config()) =
    Monitor(ccall((:sentil_monitor_parse, libsentil[]), Ptr{Cvoid},
                  (Cstring, Ptr{Cvoid}), text, _ptr(config)))

"""An owned copy of the formula the monitor watches."""
formula(m::Monitor) =
    Formula(ccall((:sentil_monitor_formula, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _ptr(m)))

"""An owned copy of the monitor's config."""
config(m::Monitor) =
    Config(ccall((:sentil_monitor_config, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _ptr(m)))

function robustness(m::Monitor, trace::Trace)
    out = Ref{Float64}(0.0)
    check_error(ccall((:sentil_monitor_robustness, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Float64}), _ptr(m), _ptr(trace), out))
    return out[]
end

function robustness_signal(m::Monitor, trace::Trace)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_monitor_robustness_signal, libsentil[]), Ptr{Float64},
                (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Csize_t}), _ptr(m), _ptr(trace), n)
    ptr == C_NULL && _last_error_code() != SENTIL_OK && _raise_last()
    return _take_doubles(ptr, n[])
end

function violations(m::Monitor, trace::Trace)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_monitor_violations, libsentil[]), Ptr{Interval},
                (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Csize_t}), _ptr(m), _ptr(trace), n)
    ptr == C_NULL && _last_error_code() != SENTIL_OK && _raise_last()
    return _take_intervals(ptr, n[])
end

"""The 1-based position of `name` in the packed update order, or `nothing`."""
function symbol_index(m::Monitor, name::AbstractString)
    idx = Ref{Csize_t}(0)
    found = Ref{Bool}(false)
    check_error(ccall((:sentil_monitor_symbol_index, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cstring, Ptr{Csize_t}, Ptr{Bool}), _ptr(m), name, idx, found))
    return found[] ? Int(idx[]) + 1 : nothing
end

"""Fold one sample given as a `name => value` mapping."""
function update!(m::Monitor, time::Real, samples::AbstractDict)
    names, vals = _split_samples(samples)
    out = Ref(Robustness(false, false, 0.0, 0.0, 0.0))
    check_error(ccall((:sentil_monitor_update, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cdouble, Ptr{Cstring}, Ptr{Float64}, Csize_t, Ptr{Robustness}),
                      _ptr(m), time, names, vals, length(names), out))
    return out[]
end

"""Fold one sample given as a vector in `symbol_index` order."""
function update_packed!(m::Monitor, time::Real, values::AbstractVector{<:Real})
    vals = convert(Vector{Float64}, values)
    out = Ref(Robustness(false, false, 0.0, 0.0, 0.0))
    check_error(ccall((:sentil_monitor_update_packed, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cdouble, Ptr{Float64}, Csize_t, Ptr{Robustness}),
                      _ptr(m), time, vals, length(vals), out))
    return out[]
end

"""Reset the monitor's streaming state to its start."""
reset!(m::Monitor) = (ccall((:sentil_monitor_reset, libsentil[]), Cvoid, (Ptr{Cvoid},), _ptr(m)); m)

export Monitor, formula, config, symbol_index, update!, update_packed!, reset!

mutable struct OnlineMonitor
    ptr::Ptr{Cvoid}
    function OnlineMonitor(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        m = new(ptr)
        finalizer(_destroy, m)
        return m
    end
end

function _destroy(m::OnlineMonitor)
    if m.ptr != C_NULL
        ccall((:sentil_stream_monitor_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), m.ptr)
        m.ptr = C_NULL
    end
end

close!(m::OnlineMonitor) = _destroy(m)

"""A streaming monitor for a formula given as text or as a borrowed `Formula`."""
OnlineMonitor(text::AbstractString) =
    OnlineMonitor(ccall((:sentil_stream_monitor_create, libsentil[]), Ptr{Cvoid}, (Cstring,), text))

OnlineMonitor(f::Formula) =
    OnlineMonitor(ccall((:sentil_stream_monitor_from_formula, libsentil[]), Ptr{Cvoid},
                        (Ptr{Cvoid},), _ptr(f)))

"""The number of distinct variables the monitor reads."""
variable_count(m::OnlineMonitor) =
    Int(ccall((:sentil_stream_monitor_variable_count, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(m)))

function symbol_index(m::OnlineMonitor, name::AbstractString)
    idx = Ref{Csize_t}(0)
    found = Ref{Bool}(false)
    check_error(ccall((:sentil_stream_monitor_symbol_index, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cstring, Ptr{Csize_t}, Ptr{Bool}), _ptr(m), name, idx, found))
    return found[] ? Int(idx[]) + 1 : nothing
end

function update!(m::OnlineMonitor, time::Real, samples::AbstractDict)
    names, vals = _split_samples(samples)
    out = Ref(Robustness(false, false, 0.0, 0.0, 0.0))
    check_error(ccall((:sentil_stream_monitor_update, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cdouble, Ptr{Cstring}, Ptr{Float64}, Csize_t, Ptr{Robustness}),
                      _ptr(m), time, names, vals, length(names), out))
    return out[]
end

function update_packed!(m::OnlineMonitor, time::Real, values::AbstractVector{<:Real})
    vals = convert(Vector{Float64}, values)
    out = Ref(Robustness(false, false, 0.0, 0.0, 0.0))
    check_error(ccall((:sentil_stream_monitor_update_packed, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cdouble, Ptr{Float64}, Csize_t, Ptr{Robustness}),
                      _ptr(m), time, vals, length(vals), out))
    return out[]
end

"""Replay a whole trace through the streaming monitor."""
function run!(m::OnlineMonitor, trace::Trace)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_stream_monitor_run, libsentil[]), Ptr{Robustness},
                (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Csize_t}), _ptr(m), _ptr(trace), n)
    ptr == C_NULL && _last_error_code() != SENTIL_OK && _raise_last()
    return _take_robustness(ptr, n[])
end

reset!(m::OnlineMonitor) =
    (ccall((:sentil_stream_monitor_reset, libsentil[]), Cvoid, (Ptr{Cvoid},), _ptr(m)); m)

export OnlineMonitor, variable_count, run!