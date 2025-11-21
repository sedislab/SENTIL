classdef SoftKind < int32
    %SOFTKIND The smoothing used for differentiable robustness.

    enumeration
        LogSumExp (0)
        ArithmeticGeometricMean (1)
    end
end