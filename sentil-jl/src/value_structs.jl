# isbits mirrors of the structs in sentil.h; field order and types must match the header.

"""A timed sample from a ring buffer."""
struct Sample
    found::Bool
    time::Float64
    value::Float64
end

"""A closed time span `[start, stop]` over which a property fails to hold."""
struct Interval
    start::Float64
    stop::Float64
end

"""A robustness verdict."""
struct Robustness
    resolved::Bool
    satisfied::Bool
    value::Float64
    lower::Float64
    upper::Float64
end

"""A confidence interval `[lower, upper]` at confidence `level`."""
struct ConfidenceInterval
    lower::Float64
    upper::Float64
    level::Float64
end

"""The width of a confidence interval."""
width(ci::ConfidenceInterval) = ci.upper - ci.lower

export Sample, Interval, Robustness, ConfidenceInterval, width

"""The outcome of a statistical model check."""
struct SmcResult
    probability::Float64
    interval::ConfidenceInterval
    satisfactions::UInt64
    samples::UInt64
    holds::Bool
end

"""Summary statistics of the robustness values seen across a sampled ensemble."""
struct RobustnessDistribution
    count::UInt64
    mean::Float64
    variance::Float64
    std_dev::Float64
    min::Float64
    max::Float64
end

"""The result of a sequential probability ratio test."""
struct SprtResult
    verdict::SprtVerdict.T
    samples::UInt64
    log_likelihood::Float64
end

"""The result of a Bayesian sequential test."""
struct BayesResult
    verdict::BayesVerdict.T
    samples::UInt64
    posterior::Float64
end

"""A rare-event estimate from multilevel splitting."""
struct RareEventResult
    probability::Float64
    violation_probability::Float64
    holds::Bool
    simulations::UInt64
end

"""A bare rare-event estimate."""
struct RareEventEstimate
    probability::Float64
    simulations::UInt64
end

"""A chance-constraint validation."""
struct ChanceReport
    estimate::Float64
    lower_bound::Float64
    samples::UInt64
    holds::Bool
end

"""The SMC settings a specification recommends."""
struct SpecSmcSettings
    confidence::Float64
    sample_budget::UInt64
end

"""The SPRT settings a specification recommends."""
struct SpecSprtSettings
    p0::Float64
    p1::Float64
    alpha::Float64
    beta::Float64
    max_samples::Csize_t
end

"""The rare-event splitting settings a specification recommends."""
struct SpecAmsSettings
    num_particles::Csize_t
    max_steps::Csize_t
end

"""A GPU splitting estimate."""
struct GpuSplittingEstimate
    violation_probability::Float64
    particles::Csize_t
    levels::UInt32
end

export SmcResult, RobustnessDistribution, SprtResult, BayesResult
export RareEventResult, RareEventEstimate, ChanceReport
export SpecSmcSettings, SpecSprtSettings, SpecAmsSettings, GpuSplittingEstimate