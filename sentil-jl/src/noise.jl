mutable struct NoiseModel <: SentilHandle
    ptr::Ptr{Cvoid}
    function NoiseModel(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        m = new(ptr)
        finalizer(_destroy, m)
        return m
    end
end

function _destroy(m::NoiseModel)
    if m.ptr != C_NULL
        ccall((:sentil_noise_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), m.ptr)
        m.ptr = C_NULL
    end
end

close!(m::NoiseModel) = _destroy(m)

"""A point mass at `value`."""
dirac(value::Real) =
    NoiseModel(ccall((:sentil_noise_dirac, libsentil[]), Ptr{Cvoid}, (Cdouble,), value))
"""A Gaussian with the given mean and standard deviation."""
gaussian(mean::Real, std_dev::Real) =
    NoiseModel(ccall((:sentil_noise_gaussian, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), mean, std_dev))
"""A uniform distribution on `[low, high]`."""
uniform(low::Real, high::Real) =
    NoiseModel(ccall((:sentil_noise_uniform, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), low, high))
"""A log-normal with underlying mean `mu` and standard deviation `sigma`."""
log_normal(mu::Real, sigma::Real) =
    NoiseModel(ccall((:sentil_noise_log_normal, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), mu, sigma))
"""An exponential with the given rate."""
exponential(lambda::Real) =
    NoiseModel(ccall((:sentil_noise_exponential, libsentil[]), Ptr{Cvoid}, (Cdouble,), lambda))
"""A gamma distribution with the given shape and scale."""
gamma(shape::Real, scale::Real) =
    NoiseModel(ccall((:sentil_noise_gamma, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), shape, scale))
"""A beta distribution with the given shape parameters."""
beta(alpha::Real, b::Real) =
    NoiseModel(ccall((:sentil_noise_beta, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), alpha, b))
"""A Weibull with the given shape and scale."""
weibull(shape::Real, scale::Real) =
    NoiseModel(ccall((:sentil_noise_weibull, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), shape, scale))
"""A Rayleigh with the given scale."""
rayleigh(scale::Real) =
    NoiseModel(ccall((:sentil_noise_rayleigh, libsentil[]), Ptr{Cvoid}, (Cdouble,), scale))
"""A Gumbel with the given location and scale."""
gumbel(location::Real, scale::Real) =
    NoiseModel(ccall((:sentil_noise_gumbel, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), location, scale))
"""A Cauchy with the given location and scale."""
cauchy(location::Real, scale::Real) =
    NoiseModel(ccall((:sentil_noise_cauchy, libsentil[]), Ptr{Cvoid}, (Cdouble, Cdouble), location, scale))
"""A Student's t with `df` degrees of freedom, shifted and scaled."""
student_t(df::Real, location::Real, scale::Real) =
    NoiseModel(ccall((:sentil_noise_student_t, libsentil[]), Ptr{Cvoid},
                     (Cdouble, Cdouble, Cdouble), df, location, scale))
"""A normal truncated to `[lower, upper]`."""
truncated_normal(mean::Real, std_dev::Real, lower::Real, upper::Real) =
    NoiseModel(ccall((:sentil_noise_truncated_normal, libsentil[]), Ptr{Cvoid},
                     (Cdouble, Cdouble, Cdouble, Cdouble), mean, std_dev, lower, upper))
"""A Poisson with the given rate."""
poisson(lambda::Real) =
    NoiseModel(ccall((:sentil_noise_poisson, libsentil[]), Ptr{Cvoid}, (Cdouble,), lambda))
"""A binomial over `n` trials each with probability `p`."""
binomial(n::Integer, p::Real) =
    NoiseModel(ccall((:sentil_noise_binomial, libsentil[]), Ptr{Cvoid}, (UInt64, Cdouble), n, p))

"""A nonparametric model that resamples the given residuals with replacement."""
function bootstrap(residuals::AbstractVector{<:Real})
    r = convert(Vector{Float64}, residuals)
    NoiseModel(ccall((:sentil_noise_bootstrap, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Csize_t), r, length(r)))
end

"""A weighted mixture of component models, which are consumed."""
function mixture(weights::AbstractVector{<:Real}, components::AbstractVector{NoiseModel})
    length(weights) == length(components) || throw(EvaluationError(SENTIL_ERR_INVALID_NOISE_MODEL,
        "$(length(weights)) weights but $(length(components)) components"))
    w = convert(Vector{Float64}, weights)
    ptrs = Ptr{Cvoid}[_ptr(m) for m in components]
    for m in components
        m.ptr = C_NULL
    end
    NoiseModel(ccall((:sentil_noise_mixture, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Ptr{Ptr{Cvoid}}, Csize_t), w, ptrs, length(components)))
end

"""Fit a Gaussian to `samples` by maximum likelihood."""
function fit_gaussian(samples::AbstractVector{<:Real})
    s = convert(Vector{Float64}, samples)
    NoiseModel(ccall((:sentil_noise_fit_gaussian, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Csize_t), s, length(s)))
end

"""Fit a nonparametric bootstrap model that resamples `samples`."""
function fit_bootstrap(samples::AbstractVector{<:Real})
    s = convert(Vector{Float64}, samples)
    NoiseModel(ccall((:sentil_noise_fit_bootstrap, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Csize_t), s, length(s)))
end

"""Fit a bootstrap model keeping at most `max_samples` reservoir-sampled points."""
function fit_bootstrap_reservoir(samples::AbstractVector{<:Real}, max_samples::Integer)
    s = convert(Vector{Float64}, samples)
    NoiseModel(ccall((:sentil_noise_fit_bootstrap_reservoir, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Csize_t, Csize_t), s, length(s), max_samples))
end

"""Fit a Gaussian mixture of `components` by expectation-maximization."""
function fit_gaussian_mixture(samples::AbstractVector{<:Real}, components::Integer, max_iters::Integer)
    s = convert(Vector{Float64}, samples)
    NoiseModel(ccall((:sentil_noise_fit_gaussian_mixture, libsentil[]), Ptr{Cvoid},
                     (Ptr{Float64}, Csize_t, Csize_t, Csize_t), s, length(s), components, max_iters))
end

"""The mean of the model, or `nothing` when it is undefined."""
function mean(m::NoiseModel)
    out = Ref{Float64}(0.0)
    ok = ccall((:sentil_noise_mean, libsentil[]), Bool, (Ptr{Cvoid}, Ptr{Float64}), _ptr(m), out)
    return ok ? out[] : nothing
end

"""The variance of the model, or `nothing` when it is undefined."""
function var(m::NoiseModel)
    out = Ref{Float64}(0.0)
    ok = ccall((:sentil_noise_variance, libsentil[]), Bool, (Ptr{Cvoid}, Ptr{Float64}), _ptr(m), out)
    return ok ? out[] : nothing
end

"""The residuals between paired ground-truth and sensor readings, additive `y - g` or multiplicative `y / g`."""
function residuals(ground_truth::AbstractVector{<:Real}, sensor::AbstractVector{<:Real};
                   interaction::NoiseInteraction.T = NoiseInteraction.Additive)
    g = convert(Vector{Float64}, ground_truth)
    s = convert(Vector{Float64}, sensor)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_noise_residuals, libsentil[]), Ptr{Float64},
                (Ptr{Float64}, Csize_t, Ptr{Float64}, Csize_t, Int32, Ptr{Csize_t}),
                g, length(g), s, length(s), Int32(interaction), n)
    ptr == C_NULL && _last_error_code() != SENTIL_OK && _raise_last()
    return _take_doubles(ptr, n[])
end

to_json(m::NoiseModel) =
    _take_string(ccall((:sentil_noise_to_json, libsentil[]), Ptr{UInt8}, (Ptr{Cvoid},), _ptr(m)))
from_json(::Type{NoiseModel}, json::AbstractString) =
    NoiseModel(ccall((:sentil_noise_from_json, libsentil[]), Ptr{Cvoid}, (Cstring,), json))

"""Load a noise model from a JSON file written by `to_json`."""
from_file(::Type{NoiseModel}, path::AbstractString) =
    NoiseModel(ccall((:sentil_noise_from_file, libsentil[]), Ptr{Cvoid}, (Cstring,), path))

export NoiseModel, dirac, gaussian, uniform, log_normal, exponential, gamma, beta
export weibull, rayleigh, gumbel, cauchy, student_t, truncated_normal, poisson
export bootstrap, mixture, fit_gaussian, fit_bootstrap, fit_bootstrap_reservoir
export fit_gaussian_mixture, residuals

mutable struct LiftingRegistry <: SentilHandle
    ptr::Ptr{Cvoid}
    function LiftingRegistry(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        r = new(ptr)
        finalizer(_destroy, r)
        return r
    end
end

function _destroy(r::LiftingRegistry)
    if r.ptr != C_NULL
        ccall((:sentil_lifting_registry_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), r.ptr)
        r.ptr = C_NULL
    end
end

close!(r::LiftingRegistry) = _destroy(r)

"""An empty registry mapping variables to noise models."""
LiftingRegistry() =
    LiftingRegistry(ccall((:sentil_lifting_registry_create, libsentil[]), Ptr{Cvoid}, ()))

"""Register a noise model for a variable, consuming the model."""
function register_noise!(r::LiftingRegistry, variable::AbstractString, model::NoiseModel;
                         interaction::NoiseInteraction.T = NoiseInteraction.Additive)
    check_error(ccall((:sentil_lifting_registry_register, libsentil[]), Int32,
                      (Ptr{Cvoid}, Cstring, Ptr{Cvoid}, Int32),
                      _ptr(r), variable, _consume!(model), Int32(interaction)))
    return r
end

function variables(r::LiftingRegistry)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_lifting_registry_variables, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(r), n)
    return _take_string_array(ptr, n[])
end

Base.isempty(r::LiftingRegistry) =
    ccall((:sentil_lifting_registry_is_empty, libsentil[]), Bool, (Ptr{Cvoid},), _ptr(r))

"""One seeded noisy realization of the trace under the registered models."""
lift(r::LiftingRegistry, trace::Trace; seed::Integer = 42) =
    Trace(ccall((:sentil_lifting_registry_lift, libsentil[]), Ptr{Cvoid},
                (Ptr{Cvoid}, Ptr{Cvoid}, UInt64), _ptr(r), _ptr(trace), seed))

export LiftingRegistry, register_noise!, lift