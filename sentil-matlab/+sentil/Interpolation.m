classdef Interpolation < int32
    %INTERPOLATION How a trace is read between its samples.

    enumeration
        Linear (0)
        Hold (1)
        Cubic (2)
    end
end