"""A sequential probability ratio test between the null probability `p0` and the alternative `p1`."""
struct SprtConfig
    p0::Float64
    p1::Float64
    alpha::Float64
    beta::Float64
    max_samples::UInt64
    seed::UInt64
end

SprtConfig(p0::Real, p1::Real; alpha::Real = 0.05, beta::Real = 0.05,
           max_samples::Integer = 100000, seed::Integer = 42) =
    SprtConfig(p0, p1, alpha, beta, max_samples, seed)

"""A Bayesian sequential test of whether the satisfaction probability is above `threshold`."""
struct BayesConfig
    threshold::Float64
    bayes_factor::Float64
    max_samples::UInt64
    seed::UInt64
end

BayesConfig(threshold::Real; bayes_factor::Real = 100.0,
            max_samples::Integer = 100000, seed::Integer = 42) =
    BayesConfig(threshold, bayes_factor, max_samples, seed)

"""Decide a `P~p` formula sequentially over the lifted ensemble."""
function check_sequential(f::Formula, trace::Trace, lifting::LiftingRegistry, config::SprtConfig)
    cfg = Ref(config)
    out = Ref{SprtResult}()
    check_error(ccall((:sentil_formula_check_sequential, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SprtConfig}, Ptr{SprtResult}),
                      _ptr(f), _ptr(trace), _ptr(lifting), cfg, out))
    return out[]
end

function check_sequential(m::Monitor, trace::Trace, lifting::LiftingRegistry, config::SprtConfig)
    cfg = Ref(config)
    out = Ref{SprtResult}()
    check_error(ccall((:sentil_monitor_check_sequential, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SprtConfig}, Ptr{SprtResult}),
                      _ptr(m), _ptr(trace), _ptr(lifting), cfg, out))
    return out[]
end

"""Decide a `P~p` formula by a Bayesian sequential test over the lifted ensemble."""
function check_bayesian(f::Formula, trace::Trace, lifting::LiftingRegistry, config::BayesConfig)
    cfg = Ref(config)
    out = Ref{BayesResult}()
    check_error(ccall((:sentil_formula_check_bayesian, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{BayesConfig}, Ptr{BayesResult}),
                      _ptr(f), _ptr(trace), _ptr(lifting), cfg, out))
    return out[]
end

export SprtConfig, BayesConfig, check_sequential, check_bayesian

mutable struct _BernoulliBox
    draw::Any
    err::Union{Nothing,Exception}
end

function _bernoulli_trampoline(ud::Ptr{Cvoid})::Bool
    box = unsafe_pointer_to_objref(ud)::_BernoulliBox
    try
        return box.draw()::Bool
    catch e
        box.err = e
        return false
    end
end

const _C_BERNOULLI = @cfunction(_bernoulli_trampoline, Bool, (Ptr{Cvoid},))

"""Run a sequential probability ratio test driven by `draw`, a `() -> Bool` source."""
function sequential_test(draw, config::SprtConfig)
    box = _BernoulliBox(draw, nothing)
    cfg = Ref(config)
    out = Ref{SprtResult}()
    GC.@preserve box begin
        code = ccall((:sentil_sequential_test, libsentil[]), Int32,
                     (Ptr{SprtConfig}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SprtResult}),
                     cfg, _C_BERNOULLI, pointer_from_objref(box), out)
    end
    box.err === nothing || throw(box.err)
    check_error(code)
    return out[]
end

"""Run a Bayesian sequential test driven by `draw`, a `() -> Bool` source."""
function bayes_sequential_test(draw, config::BayesConfig)
    box = _BernoulliBox(draw, nothing)
    cfg = Ref(config)
    out = Ref{BayesResult}()
    GC.@preserve box begin
        code = ccall((:sentil_bayes_sequential_test, libsentil[]), Int32,
                     (Ptr{BayesConfig}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{BayesResult}),
                     cfg, _C_BERNOULLI, pointer_from_objref(box), out)
    end
    box.err === nothing || throw(box.err)
    check_error(code)
    return out[]
end

export sequential_test, bayes_sequential_test