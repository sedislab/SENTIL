classdef SprtVerdict < int32
    %SPRTVERDICT The outcome of a sequential probability ratio test.

    enumeration
        AcceptH0 (0)
        AcceptH1 (1)
        Inconclusive (2)
    end
end