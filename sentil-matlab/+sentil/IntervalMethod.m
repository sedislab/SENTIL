classdef IntervalMethod < int32
    %INTERVALMETHOD A binomial confidence interval estimator.

    enumeration
        Wilson (0)
        ClopperPearson (1)
        Jeffreys (2)
        AgrestiCoull (3)
    end
end