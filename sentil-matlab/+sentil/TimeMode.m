classdef TimeMode < int32
    %TIMEMODE Whether a monitor reads its trace on the sample grid or in dense time.

    enumeration
        Discrete (0)
        Dense (1)
    end
end