classdef SmoothConfig
    %SMOOTHCONFIG Settings for differentiable robustness.

    properties
        temperature (1, 1) double = 10
        kind = sentil.SoftKind.LogSumExp
    end
end