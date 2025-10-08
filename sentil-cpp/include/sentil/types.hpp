// Value types and scoped enums. Each enumerator is pinned to its C constant, 
// so converting to and from the C enum is an exact cast. The owning RAII classes
// live in sentil.hpp.
#ifndef SENTIL_TYPES_HPP
#define SENTIL_TYPES_HPP

#include <sentil.h>

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

}  // namespace sentil

#endif  // SENTIL_TYPES_HPP