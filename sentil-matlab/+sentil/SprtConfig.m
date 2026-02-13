classdef SprtConfig
    %SPRTCONFIG Settings for a sequential probability ratio test.

    properties
        p0 (1, 1) double
        p1 (1, 1) double
        alpha (1, 1) double = 0.05
        beta (1, 1) double = 0.05
        max_samples (1, 1) double = 100000
        seed (1, 1) double = 42
    end

    methods
        function obj = SprtConfig(p0, p1)
            %SPRTCONFIG A config for the hypotheses p0 and p1.
            if nargin >= 1
                obj.p0 = p0;
            end
            if nargin >= 2
                obj.p1 = p1;
            end
        end
    end
end