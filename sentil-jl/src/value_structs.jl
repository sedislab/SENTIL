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