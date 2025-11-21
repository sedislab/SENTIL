"""The Wilson score interval for `successes` out of `trials` at confidence `level`."""
wilson_interval(successes::Integer, trials::Integer, level::Real) =
    ccall((:sentil_wilson_interval, libsentil[]), ConfidenceInterval,
          (UInt64, UInt64, Cdouble), successes, trials, level)

"""The Clopper-Pearson exact interval, the conservative choice."""
clopper_pearson(successes::Integer, trials::Integer, level::Real) =
    ccall((:sentil_clopper_pearson, libsentil[]), ConfidenceInterval,
          (UInt64, UInt64, Cdouble), successes, trials, level)

"""The Jeffreys interval."""
jeffreys_interval(successes::Integer, trials::Integer, level::Real) =
    ccall((:sentil_jeffreys_interval, libsentil[]), ConfidenceInterval,
          (UInt64, UInt64, Cdouble), successes, trials, level)

"""The Agresti-Coull interval."""
agresti_coull(successes::Integer, trials::Integer, level::Real) =
    ccall((:sentil_agresti_coull, libsentil[]), ConfidenceInterval,
          (UInt64, UInt64, Cdouble), successes, trials, level)

"""A confidence interval over a Bernoulli count, by the chosen estimator."""
function interval(successes::Integer, trials::Integer, level::Real;
                  method::IntervalMethod.T = IntervalMethod.Wilson)
    if method == IntervalMethod.Wilson
        wilson_interval(successes, trials, level)
    elseif method == IntervalMethod.ClopperPearson
        clopper_pearson(successes, trials, level)
    elseif method == IntervalMethod.Jeffreys
        jeffreys_interval(successes, trials, level)
    else
        agresti_coull(successes, trials, level)
    end
end

"""The z-score, the standard-normal quantile, for a two-sided confidence `level`."""
z_score(level::Real) = ccall((:sentil_z_score, libsentil[]), Cdouble, (Cdouble,), level)

"""The sample count that bounds the estimate's error by `epsilon` with confidence `1 - delta`."""
function chernoff_hoeffding_samples(epsilon::Real, delta::Real)
    out = Ref{UInt64}(0)
    check_error(ccall((:sentil_chernoff_hoeffding_samples, libsentil[]), Int32,
                      (Cdouble, Cdouble, Ptr{UInt64}), epsilon, delta, out))
    return Int(out[])
end

"""The sample count for a target half-width `epsilon` of the Wilson interval at `level`."""
function wilson_samples(epsilon::Real, level::Real)
    out = Ref{UInt64}(0)
    check_error(ccall((:sentil_wilson_samples, libsentil[]), Int32,
                      (Cdouble, Cdouble, Ptr{UInt64}), epsilon, level, out))
    return Int(out[])
end

export wilson_interval, clopper_pearson, jeffreys_interval, agresti_coull, interval
export z_score, chernoff_hoeffding_samples, wilson_samples

"""Configuration for statistical model checking."""
struct SmcConfig
    samples::UInt64
    confidence::Float64
    seed::UInt64
    interval_method::IntervalMethod.T
end

SmcConfig(; samples::Integer = 10000, confidence::Real = 0.95, seed::Integer = 42,
          method::IntervalMethod.T = IntervalMethod.Wilson) =
    SmcConfig(samples, confidence, seed, method)

"""Estimate the satisfaction probability of a `P~p` formula by sampling the lifted trace ensemble."""
function check(f::Formula, trace::Trace, lifting::LiftingRegistry; config::SmcConfig = SmcConfig())
    cfg = Ref(config)
    out = Ref{SmcResult}()
    check_error(ccall((:sentil_formula_check, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcConfig}, Ptr{SmcResult}),
                      _ptr(f), _ptr(trace), _ptr(lifting), cfg, out))
    return out[]
end

"""Like `check`, but always with the Clopper-Pearson exact interval."""
function check_conservative(f::Formula, trace::Trace, lifting::LiftingRegistry; config::SmcConfig = SmcConfig())
    cfg = Ref(config)
    out = Ref{SmcResult}()
    check_error(ccall((:sentil_formula_check_conservative, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcConfig}, Ptr{SmcResult}),
                      _ptr(f), _ptr(trace), _ptr(lifting), cfg, out))
    return out[]
end

"""Like `check`, and also returns the distribution of robustness values it saw."""
function check_distribution(f::Formula, trace::Trace, lifting::LiftingRegistry; config::SmcConfig = SmcConfig())
    cfg = Ref(config)
    out = Ref{SmcResult}()
    dist = Ref{RobustnessDistribution}()
    check_error(ccall((:sentil_formula_check_distribution, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcConfig},
                       Ptr{SmcResult}, Ptr{RobustnessDistribution}),
                      _ptr(f), _ptr(trace), _ptr(lifting), cfg, out, dist))
    return out[], dist[]
end

"""Estimate the satisfaction probability of a monitor's probabilistic formula."""
function check(m::Monitor, trace::Trace, lifting::LiftingRegistry)
    out = Ref{SmcResult}()
    check_error(ccall((:sentil_monitor_check, libsentil[]), Int32,
                      (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcResult}),
                      _ptr(m), _ptr(trace), _ptr(lifting), out))
    return out[]
end

export SmcConfig, check, check_conservative, check_distribution

"""A streaming monitor that tracks a `P~p` formula online, lifting each reading through the registry."""
function OnlineMonitor(f::Formula, lifting::LiftingRegistry; config::SmcConfig = SmcConfig())
    cfg = Ref(config)
    OnlineMonitor(ccall((:sentil_stream_monitor_with_lifting, libsentil[]), Ptr{Cvoid},
                        (Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcConfig}), _ptr(f), _ptr(lifting), cfg))
end

"""Add a `P~p` formula to a multi-monitor, tracked online through a lifted ensemble."""
function add_probabilistic!(m::MultiMonitor, id::AbstractString, f::Formula,
                            lifting::LiftingRegistry; config::SmcConfig = SmcConfig())
    cfg = Ref(config)
    check_error(ccall((:sentil_multi_monitor_add_probabilistic, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cstring, Ptr{Cvoid}, Ptr{Cvoid}, Ptr{SmcConfig}),
                      _ptr(m), id, _ptr(f), _ptr(lifting), cfg))
    return m
end

export add_probabilistic!