"How time between samples is read."
module TimeMode
@enum T::Int32 Discrete = 0 Dense = 1
end

"Interpolation between dense-time samples."
module Interpolation
@enum T::Int32 Linear = 0 Hold = 1 Cubic = 2
end

"The confidence interval estimator for a satisfaction probability."
module IntervalMethod
@enum T::Int32 Wilson = 0 ClopperPearson = 1 Jeffreys = 2 AgrestiCoull = 3
end

"How sensor noise combines with the true value."
module NoiseInteraction
@enum T::Int32 Additive = 0 Multiplicative = 1
end

"The decision a sequential probability ratio test reaches."
module SprtVerdict
@enum T::Int32 AcceptH0 = 0 AcceptH1 = 1 Inconclusive = 2
end

"The decision a Bayesian sequential test reaches."
module BayesVerdict
@enum T::Int32 Holds = 0 Fails = 1 Inconclusive = 2
end

"The soft semantics used for differentiable robustness in synthesis."
module SoftKind
@enum T::Int32 LogSumExp = 0 ArithmeticGeometricMean = 1
end

"Which solver open-loop synthesis uses."
module Backend
@enum T::Int32 Auto = 0 Gradient = 1 CmaEs = 2 Milp = 3
end

"The direction of a probabilistic operator `P~p`."
module ProbabilityOp
@enum T::Int32 Ge = 0 Gt = 1 Le = 2 Lt = 3
end

export TimeMode, Interpolation, IntervalMethod, NoiseInteraction, SprtVerdict
export BayesVerdict, SoftKind, Backend, ProbabilityOp