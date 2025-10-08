// Value types and scoped enums. Each enumerator is pinned to its C constant, 
// so converting to and from the C enum is an exact cast. The owning RAII classes
// live in sentil.hpp.
#ifndef SENTIL_TYPES_HPP
#define SENTIL_TYPES_HPP

#include <sentil.h>

#include <cstddef>
#include <cstdint>

namespace sentil {

/// Whether a monitor reads the sample grid (discrete) or catches threshold crossings between samples (dense).
enum class TimeMode {
    Discrete = SENTIL_TIME_DISCRETE,
    Dense = SENTIL_TIME_DENSE,
};

/// How a trace fills values between its samples when resampling.
enum class Interpolation {
    Linear = SENTIL_INTERP_LINEAR,
    ZeroOrderHold = SENTIL_INTERP_HOLD,
    CubicSpline = SENTIL_INTERP_CUBIC,
};

/// The binomial confidence interval estimator.
enum class IntervalMethod {
    Wilson = SENTIL_WILSON,
    ClopperPearson = SENTIL_CLOPPER_PEARSON,
    Jeffreys = SENTIL_JEFFREYS,
    AgrestiCoull = SENTIL_AGRESTI_COULL,
};

/// Whether the noise is an additive residual y - g or a multiplicative one y / g.
enum class NoiseInteraction {
    Additive = SENTIL_NOISE_ADDITIVE,
    Multiplicative = SENTIL_NOISE_MULTIPLICATIVE,
};

/// The verdict of a sequential probability ratio test.
enum class SprtVerdict {
    AcceptH0 = SENTIL_SPRT_ACCEPT_H0,
    AcceptH1 = SENTIL_SPRT_ACCEPT_H1,
    Inconclusive = SENTIL_SPRT_INCONCLUSIVE,
};

/// The verdict of a Bayesian sequential test.
enum class BayesVerdict {
    Holds = SENTIL_BAYES_HOLDS,
    Fails = SENTIL_BAYES_FAILS,
    Inconclusive = SENTIL_BAYES_INCONCLUSIVE,
};

/// The soft min and max used by smooth robustness.
enum class SoftKind {
    LogSumExp = SENTIL_SOFT_LOG_SUM_EXP,
    ArithmeticGeometricMean = SENTIL_SOFT_ARITHMETIC_GEOMETRIC_MEAN,
};

/// The optimization backend open-loop synthesis chooses or is told to use.
enum class Backend {
    Auto = SENTIL_BACKEND_AUTO,
    Gradient = SENTIL_BACKEND_GRADIENT,
    CmaEs = SENTIL_BACKEND_CMA_ES,
    Milp = SENTIL_BACKEND_MILP,
};

/// The comparison in a predicate f(x) ~ c.
enum class ComparisonOp {
    Lt = SENTIL_CMP_LT,
    Le = SENTIL_CMP_LE,
    Gt = SENTIL_CMP_GT,
    Ge = SENTIL_CMP_GE,
    Eq = SENTIL_CMP_EQ,
    Ne = SENTIL_CMP_NE,
};

/// A binary arithmetic operator inside an expression.
enum class BinaryOp {
    Add = SENTIL_BIN_ADD,
    Sub = SENTIL_BIN_SUB,
    Mul = SENTIL_BIN_MUL,
    Div = SENTIL_BIN_DIV,
    Mod = SENTIL_BIN_MOD,
    Pow = SENTIL_BIN_POW,
};

/// The threshold direction of a probabilistic operator P~p(phi).
enum class ProbabilityOp {
    Ge = SENTIL_PROB_GE,
    Gt = SENTIL_PROB_GT,
    Le = SENTIL_PROB_LE,
    Lt = SENTIL_PROB_LT,
};

/// A robustness verdict.
struct Robustness {
    bool resolved;
    bool satisfied;
    double value;
    double lower;
    double upper;
};

/// A time span [start, end] where a property does not hold.
struct Interval {
    double start;
    double end;
};

/// A timed sample.
struct Sample {
    bool found;
    double time;
    double value;
};

/// A binomial proportion confidence interval at a confidence level.
struct ConfidenceInterval {
    double lower;
    double upper;
    double level;

    double width() const { return upper - lower; }
};

/// Statistical model checking settings.
struct SmcConfig {
    std::uint64_t samples = 10000;
    double confidence = 0.95;
    std::uint64_t seed = 42;
    IntervalMethod method = IntervalMethod::Wilson;
};

/// Sequential probability ratio test settings.
struct SprtConfig {
    double p0;
    double p1;
    double alpha = 0.05;
    double beta = 0.05;
    std::uint64_t max_samples = 100000;
    std::uint64_t seed = 42;
};

/// Bayesian sequential test settings.
struct BayesConfig {
    double threshold;
    double bayes_factor = 100.0;
    std::uint64_t max_samples = 100000;
    std::uint64_t seed = 42;
};

/// Rare-event splitting settings.
struct RareEventConfig {
    std::size_t particles = 4096;
    double margin = 0.0;
    std::uint64_t seed = 42;
};

/// Smooth-robustness settings.
struct SmoothConfig {
    double temperature = 10.0;
    SoftKind kind = SoftKind::LogSumExp;
};

/// CMA-ES settings for the falsifier and the black-box backend.
struct CmaConfig {
    std::size_t population = 0;
    std::size_t max_generations = 300;
    double initial_step = 0.3;
    double tol_step = 1e-11;
    std::uint64_t seed = 42;
};

namespace detail {

inline sentil_smc_config_t to_c(const SmcConfig& c) {
    return sentil_smc_config_t{c.samples, c.confidence, c.seed,
                               static_cast<sentil_interval_method_t>(c.method)};
}
inline sentil_sprt_config_t to_c(const SprtConfig& c) {
    return sentil_sprt_config_t{c.p0, c.p1, c.alpha, c.beta, c.max_samples, c.seed};
}
inline sentil_bayes_config_t to_c(const BayesConfig& c) {
    return sentil_bayes_config_t{c.threshold, c.bayes_factor, c.max_samples, c.seed};
}
inline sentil_rare_event_config_t to_c(const RareEventConfig& c) {
    return sentil_rare_event_config_t{c.particles, c.margin, c.seed};
}
inline sentil_smooth_config_t to_c(const SmoothConfig& c) {
    return sentil_smooth_config_t{c.temperature, static_cast<sentil_soft_kind_t>(c.kind)};
}
inline sentil_cma_config_t to_c(const CmaConfig& c) {
    return sentil_cma_config_t{c.population, c.max_generations, c.initial_step, c.tol_step, c.seed};
}

inline Robustness from_c(const sentil_robustness_t& r) {
    return Robustness{r.resolved, r.satisfied, r.value, r.lower, r.upper};
}
inline Interval from_c(const sentil_interval_t& i) { return Interval{i.start, i.end}; }
inline Sample from_c(const sentil_sample_t& s) { return Sample{s.found, s.time, s.value}; }
inline ConfidenceInterval from_c(const sentil_confidence_interval_t& c) {
    return ConfidenceInterval{c.lower, c.upper, c.level};
}

}  // namespace detail

}  // namespace sentil

#endif  // SENTIL_TYPES_HPP