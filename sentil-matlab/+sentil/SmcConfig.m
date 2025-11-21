classdef SmcConfig
    %SMCCONFIG Settings for statistical model checking.

    properties
        samples (1, 1) double = 10000
        confidence (1, 1) double = 0.95
        seed (1, 1) double = 42
        method = sentil.IntervalMethod.Wilson
    end
end