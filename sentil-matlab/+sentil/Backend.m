classdef Backend < int32
    %BACKEND The solver open-loop synthesis chooses.

    enumeration
        Auto (0)
        Gradient (1)
        CmaEs (2)
        Milp (3)
    end
end